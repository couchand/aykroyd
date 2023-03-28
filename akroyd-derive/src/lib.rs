use quote::quote;

#[proc_macro_derive(FromRow)]
pub fn derive_from_row(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let body = match ast.data {
        syn::Data::Enum(_) => panic!("Cannot derive FromRow on enum!"),
        syn::Data::Union(_) => panic!("Cannot derive FromRow on union!"),
        syn::Data::Struct(s) => match s.fields {
            syn::Fields::Named(fs) => {
                let (indices, fields): (Vec<_>, Vec<_>) = fs
                    .named
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        let i = syn::Index::from(i);
                        let f = &f.ident;
                        (quote!(#i), quote!(#f))
                    })
                    .unzip();

                quote! {
                    #name {
                        #(
                            #fields : row.get(#indices),
                        )*
                    }
                }
            }
            syn::Fields::Unnamed(fs) => {
                let indices: Vec<_> = fs
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, _)| {
                        let i = syn::Index::from(i);
                        quote!(#i)
                    })
                    .collect();

                quote! {
                    #name(
                        #(
                            row.get(#indices),
                        )*
                    )
                }
            }
            syn::Fields::Unit => quote!(#name),
        },
    };

    proc_macro::TokenStream::from(quote! {
        #[automatically_derived]
        impl ::akroyd::FromRow for #name {
            fn from_row(row: &::akroyd::types::Row) -> Self {
                #body
            }
        }
    })
}

#[proc_macro_derive(ToRow)]
pub fn derive_to_row(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let fields = match ast.data {
        syn::Data::Enum(_) => panic!("Cannot derive ToRow on enum!"),
        syn::Data::Union(_) => panic!("Cannot derive ToRow on union!"),
        syn::Data::Struct(s) => match s.fields {
            syn::Fields::Named(fs) => fs
                .named
                .iter()
                .map(|f| {
                    let f = &f.ident;
                    quote!(#f)
                })
                .collect(),
            syn::Fields::Unnamed(fs) => fs
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let i = syn::Index::from(i);
                    quote!(#i)
                })
                .collect(),
            syn::Fields::Unit => vec![],
        },
    };

    proc_macro::TokenStream::from(quote! {
        #[automatically_derived]
        impl ::akroyd::ToRow for #name {
            fn to_row(&self) -> Vec<&(dyn ::akroyd::types::ToSql + Sync)> {
                let mut res = vec![];

                #(
                    res.push(&self.#fields as &(dyn ::akroyd::types::ToSql + Sync));
                )*

                res
            }
        }
    })
}

#[proc_macro_derive(Query, attributes(query))]
pub fn derive_query(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_query_impl(input, quote!(::akroyd::Query), "results")
}

#[proc_macro_derive(QueryOne, attributes(query))]
pub fn derive_query_one(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_query_impl(input, quote!(::akroyd::QueryOne), "result")
}

fn derive_query_impl(input: proc_macro::TokenStream, trait_name: proc_macro2::TokenStream, results_attr: &str) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let mut output = None;
    let mut query = None;

    for attr in ast.attrs {
        let ident = match attr.path.get_ident() {
            Some(ident) => ident,
            None => continue,
        };

        if ident == "query" {
            match attr.parse_meta().expect("Unable to parse query attribute!") {
                syn::Meta::List(list) => {
                    for item in list.nested.iter() {
                        match &item {
                            syn::NestedMeta::Meta(syn::Meta::NameValue(pair)) => {
                                let param = match pair.path.get_ident() {
                                    Some(ident) => ident,
                                    None => unreachable!("namevalue always has ident!"),
                                };

                                match param.to_string().as_ref() {
                                    "text" => {
                                        let lit = &pair.lit;
                                        query = Some(quote!(#lit));
                                    }
                                    "file" => {
                                        let filename = &pair.lit;
                                        query = Some(quote!(include_str!(#filename)));
                                    }
                                    _ => {
                                        panic!("Unknown Query derive parameter: {:?}", param);
                                    }
                                }
                            }
                            syn::NestedMeta::Meta(syn::Meta::List(list)) => {
                                let param = match list.path.get_ident() {
                                    Some(ident) => ident,
                                    None => unreachable!("metalist always has ident!"),
                                };

                                if param == results_attr {
                                    if list.nested.len() != 1 {
                                        panic!(
                                            "Expected a single result type in Query derive!"
                                        );
                                    }

                                    match list.nested.first().unwrap() {
                                        syn::NestedMeta::Meta(syn::Meta::Path(path)) => {
                                            output = Some(path.clone());
                                        }
                                        _ => {
                                            panic!("Expected a single result type in Query derive!");
                                        }
                                    }
                                } else {
                                    panic!("Unknown Query derive parameter: {:?}", param);
                                }
                            }
                            _ => {
                                panic!("Unsupported Query derive parameters");
                            }
                        }
                    }
                }
                _ => {
                    panic!("Unsupported Query derive parameters");
                }
            }
        }
    }

    let output = output.expect("Unable to find output result type attribute for Query derive!");
    let query = query.expect("Unable to find query text or file attribute for Query derive!");

    proc_macro::TokenStream::from(quote! {
        #[automatically_derived]
        impl #trait_name for #name {
            type Output = #output;
            const TEXT: &'static str = #query;
        }
    })
}
