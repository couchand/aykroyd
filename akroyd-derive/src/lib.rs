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
                    .map(|f| {
                        let f = f.ident.as_ref().unwrap();
                        let i = f.to_string();
                        (quote!(#i), quote!(#f))
                    })
                    .unzip();

                quote! {
                    Ok(#name {
                        #(
                            #fields : row.try_get(#indices)?,
                        )*
                    })
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
                    Ok(#name(
                        #(
                            row.try_get(#indices)?,
                        )*
                    ))
                }
            }
            syn::Fields::Unit => quote!(Ok(#name)),
        },
    };

    proc_macro::TokenStream::from(quote! {
        #[automatically_derived]
        impl ::akroyd::FromRow for #name {
            fn from_row(row: ::akroyd::types::Row) -> ::std::result::Result<Self, ::akroyd::types::Error> {
                #body
            }
        }
    })
}

#[proc_macro_derive(Query, attributes(query))]
pub fn derive_query(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_query_impl(input, quote!(::akroyd::Query), "row")
}

#[proc_macro_derive(QueryOne, attributes(query))]
pub fn derive_query_one(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_query_impl(input, quote!(::akroyd::QueryOne), "row")
}

fn derive_query_impl(
    input: proc_macro::TokenStream,
    trait_name: proc_macro2::TokenStream,
    results_attr: &str,
) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let generics = &ast.generics;
    let mut output = None;
    let mut query = None;

    for attr in ast.attrs {
        if attr.path().is_ident("query") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("text") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    query = Some(quote!(#lit));
                    return Ok(());
                }
                if meta.path.is_ident("file") {
                    let value = meta.value()?;
                    let filename: syn::LitStr = value.parse()?;
                    query = Some(quote!(include_str!(#filename)));
                    return Ok(());
                }
                if meta.path.is_ident(results_attr) {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let ty: syn::Type = content.parse()?;
                    output = Some(ty);
                    return Ok(());
                }
                Err(meta.error("unrecognized attribute"))
            })
            .expect("Unable to parse query attribute!");
        }
    }

    let fields = match ast.data {
        syn::Data::Enum(_) => panic!("Cannot derive Statement on enum!"),
        syn::Data::Union(_) => panic!("Cannot derive Statement on union!"),
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

    let output = output.expect("Unable to find output result type attribute for Query derive!");
    let query = query.expect("Unable to find query text or file attribute for Query derive!");

    proc_macro::TokenStream::from(quote! {
        #[automatically_derived]
        impl #generics ::akroyd::Statement for #name #generics {
            const TEXT: &'static str = #query;

            fn to_row(&self) -> Vec<&(dyn ::akroyd::types::ToSql + Sync)> {
                let mut res = vec![];

                #(
                    res.push(&self.#fields as &(dyn ::akroyd::types::ToSql + Sync));
                )*

                res
            }
        }

        #[automatically_derived]
        impl #generics #trait_name for #name #generics {
            type Row = #output;
        }
    })
}

#[proc_macro_derive(Statement, attributes(query))]
pub fn derive_exeucte(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let generics = &ast.generics;
    let mut query = None;

    for attr in ast.attrs {
        if attr.path().is_ident("query") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("text") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    query = Some(quote!(#lit));
                    return Ok(());
                }
                if meta.path.is_ident("file") {
                    let value = meta.value()?;
                    let filename: syn::LitStr = value.parse()?;
                    query = Some(quote!(include_str!(#filename)));
                    return Ok(());
                }
                Err(meta.error("unrecognized attribute"))
            })
            .expect("Unable to parse query attribute!");
        }
    }

    let fields = match ast.data {
        syn::Data::Enum(_) => panic!("Cannot derive Statement on enum!"),
        syn::Data::Union(_) => panic!("Cannot derive Statement on union!"),
        syn::Data::Struct(s) => match s.fields {
            syn::Fields::Named(fs) => fs
                .named
                .iter()
                .map(|f| {
                    let index = parse_field_attrs(&f.attrs);
                    let f = &f.ident;
                    (index, quote!(#f))
                })
                .collect(),
            syn::Fields::Unnamed(fs) => fs
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let index = parse_field_attrs(&f.attrs);
                    let i = syn::Index::from(i);
                    (index, quote!(#i))
                })
                .collect(),
            syn::Fields::Unit => vec![],
        },
    };

    let mut next_index = 1;
    let (indexes, fields): (Vec<_>, Vec<_>) = fields.into_iter()
        .map(|(index, field)| match index {
            Some(i) => (i, field),
            None => {
                let i = next_index;
                next_index += 1;
                (i, field)
            }
        })
        .unzip();

    let mut sorted = std::iter::repeat(None).take(fields.len()).collect::<Vec<_>>();

    for (i, index) in indexes.into_iter().enumerate() {
        assert!(index > 0);
        assert!(index <= fields.len());

        sorted[index - 1] = Some(fields[i].clone());
    }

    let fields = sorted
        .into_iter()
        .map(|f| f.unwrap())
        .collect::<Vec<_>>();

    let query = query.expect("Unable to find query text or file attribute for Query derive!");

    proc_macro::TokenStream::from(quote! {
        #[automatically_derived]
        impl #generics ::akroyd::Statement for #name #generics {
            const TEXT: &'static str = #query;

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

fn parse_field_attrs(attrs: &[syn::Attribute]) -> Option<usize> {
    let mut index = None;
    for attr in attrs {
        if attr.path().is_ident("query") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("param") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    let text = lit.value();
                    if text.chars().next() != Some('$') {
                        return Err(meta.error("Parameter must be an integer prefixed with $"));
                    }
                    let i: u8 = text.chars().skip(1).collect::<String>().parse().or(
                        Err(meta.error("Parameter must be an integer prefixed with $"))
                    )?;
                    if i == 0 {
                        return Err(meta.error("Parameter must be an integer prefixed with $"));
                    }
                    index = Some(i as usize);
                    return Ok(());
                }
                Err(meta.error("unrecognized attribute"))
            })
            .expect("Unable to parse query attribute!");
        }
    }
    index
}
