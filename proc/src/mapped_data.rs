use std::{collections::BTreeMap, mem};

use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    Error, Field, FieldMutability, Fields, FieldsNamed, ItemStruct, LitInt, Result, Visibility,
    parse_quote, token::Colon,
};

#[derive(Debug)]
pub struct Mapped {
    repr: TokenStream,
    item: ItemStruct,
    asserts: Vec<TokenStream>,
}

impl Mapped {
    pub fn parse(attr: TokenStream, item: TokenStream) -> Result<Self> {
        let repr = quote! { #[repr(C)] };

        let mut item = syn::parse2::<ItemStruct>(item)?;

        let Fields::Named(fields) = &mut item.fields else {
            return Err(Error::new_spanned(
                item.fields,
                "only named fields are supported",
            ));
        };

        let mut asserts = vec![];
        let mut field_groups: BTreeMap<usize, Vec<Field>> = BTreeMap::new();
        {
            let mut original_fields: FieldsNamed = parse_quote!({});
            mem::swap(fields, &mut original_fields);

            let mut offset = 0;

            for mut field in original_fields.named {
                if let Some(index) = field.attrs.iter().position(|x| x.path().is_ident("offset")) {
                    let attr = field.attrs.remove(index);
                    let lit = attr.parse_args::<LitInt>()?;
                    offset = lit.base10_parse::<usize>()?;
                    if field_groups.contains_key(&offset) {
                        return Err(Error::new_spanned(attr, "duplicate offset"));
                    }
                }

                if let Some(repeat) = field.attrs.iter().find(|x| x.path().is_ident("offset")) {
                    return Err(Error::new_spanned(repeat, "duplicate offset attribute"));
                }

                if let Some(entry) = field_groups.get_mut(&offset) {
                    entry.push(field);
                } else {
                    let item_ident = &item.ident;
                    let field_ident = &field.ident;
                    asserts.push(quote! {
                        assert!(::std::mem::offset_of!(#item_ident, #field_ident) == #offset);
                    });
                    field_groups.insert(offset, vec![field]);
                }
            }

            if !attr.is_empty() {
                let size = syn::parse2::<LitInt>(attr)?.base10_parse::<usize>()?;
                if let Some(fields) = field_groups.insert(
                    size,
                    vec![Field {
                        attrs: vec![],
                        vis: Visibility::Inherited,
                        ident: Some(format_ident!("_end")),
                        colon_token: Some(Colon::default()),
                        ty: parse_quote!(()),
                        mutability: FieldMutability::None,
                    }],
                ) && let Some(first) = fields.first()
                {
                    return Err(Error::new_spanned(first, "field at size offset"));
                }

                let item_ident = &item.ident;
                asserts.push(quote! {
                    assert!(::std::mem::size_of::<#item_ident>() == #size);
                });
            }
        }

        let mut len = None;
        for (index, (group_offset, group_fields)) in field_groups.into_iter().enumerate() {
            if group_offset != 0 {
                let mut statements = vec![quote! { let offset = #group_offset; }];

                {
                    let len = len.unwrap_or_else(|| quote! { 0usize });
                    let overlapping_err = format!("overlapping offset: {group_offset:#x}");
                    statements.push(quote! {
                        let len = #len;
                        assert!(
                            len <= offset,
                            #overlapping_err
                        );
                    });
                }

                {
                    let field = group_fields
                        .first()
                        .expect("group should have at least one field");
                    let ty = &field.ty;
                    let unaligned_err = format!("unaligned offset: {group_offset:#x}");
                    statements.push(quote! {
                        assert!(
                            offset.is_multiple_of(align_of::<#ty>()),
                            #unaligned_err
                        );
                    });
                }

                statements.push(quote! { offset - len });

                let pad_ident = format_ident!("_padding{index}");
                fields.named.push(parse_quote! {
                    #pad_ident: [u8; {
                        #(#statements)*
                    }]
                });
            }

            {
                let last = group_fields.last().expect("group should have fields");
                let last_ident = &last.ident;
                let last_ty = &last.ty;
                len = Some(quote! {{
                    #repr
                    struct Inner {
                        #(#group_fields),*
                    }
                    #group_offset
                        + ::std::mem::offset_of!(Inner, #last_ident)
                        + size_of::<#last_ty>()
                }});
            }

            for field in group_fields {
                fields.named.push(field);
            }
        }

        Ok(Self {
            repr,
            item,
            asserts,
        })
    }
}

impl ToTokens for Mapped {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let repr = &self.repr;
        let item = &self.item;
        let asserts = &self.asserts;
        tokens.extend(quote! {
            #repr
            #item

            const _: () = {
                #(#asserts)*
            };
        });
    }
}
