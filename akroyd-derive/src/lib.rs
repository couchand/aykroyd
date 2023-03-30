use quote::quote;

/// Derive macro available if akroyd is built with `features = ["derive"]`.
#[proc_macro_derive(FromRow, attributes(query))]
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
                        let i = match parse_field_column_attr(&f.attrs) {
                            None => {
                                let i = f.ident.as_ref().unwrap().to_string();
                                quote!(#i)
                            }
                            Some(i) => quote!(#i),
                        };
                        let f = f.ident.as_ref().unwrap();
                        (i, quote!(#f))
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
                    .map(|(i, f)| match parse_field_column_attr(&f.attrs) {
                        None => {
                            let i = syn::Index::from(i);
                            quote!(#i)
                        }
                        Some(i) => quote!(#i),
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

/// Derive macro available if akroyd is built with `features = ["derive"]`.
#[proc_macro_derive(Query, attributes(query))]
pub fn derive_query(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_query_impl(input, quote!(::akroyd::Query))
}

/// Derive macro available if akroyd is built with `features = ["derive"]`.
#[proc_macro_derive(QueryOne, attributes(query))]
pub fn derive_query_one(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_query_impl(input, quote!(::akroyd::QueryOne))
}

fn derive_query_impl(
    input: proc_macro::TokenStream,
    trait_name: proc_macro2::TokenStream,
) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let generics = &ast.generics;

    let statement_impl = derive_statement_impl(&ast);

    proc_macro::TokenStream::from(quote! {
        #statement_impl

        #[automatically_derived]
        impl #generics #trait_name for #name #generics {}
    })
}

/// Derive macro available if akroyd is built with `features = ["derive"]`.
#[proc_macro_derive(Statement, attributes(query))]
pub fn derive_statement(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    derive_statement_impl(&ast).into()
}

fn derive_statement_impl(ast: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    let generics = &ast.generics;

    let params = parse_struct_attrs(&ast.attrs);

    let query = if let Some(text) = &params.text {
        quote!(#text)
    } else if let Some(file) = &params.file {
        quote!(include_str!(#file))
    } else {
        panic!("Unable to find query text or file attribute for Query derive!");
    };
    let output = if let Some(row) = &params.row {
        quote!(#row)
    } else {
        quote!(())
    };

    let fields = match &ast.data {
        syn::Data::Enum(_) => panic!("Cannot derive Statement on enum!"),
        syn::Data::Union(_) => panic!("Cannot derive Statement on union!"),
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Named(fs) => fs
                .named
                .iter()
                .map(|f| {
                    let index = parse_field_param_attr(&f.attrs);
                    let f = &f.ident;
                    (index, quote!(#f))
                })
                .collect(),
            syn::Fields::Unnamed(fs) => fs
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let index = parse_field_param_attr(&f.attrs);
                    let i = syn::Index::from(i);
                    (index, quote!(#i))
                })
                .collect(),
            syn::Fields::Unit => vec![],
        },
    };

    let mut next_index = 1;
    let (indexes, fields): (Vec<_>, Vec<_>) = fields
        .into_iter()
        .map(|(index, field)| match index {
            Some(i) => (i, field),
            None => {
                let i = next_index;
                next_index += 1;
                (i, field)
            }
        })
        .unzip();

    let mut sorted = std::iter::repeat(None)
        .take(fields.len())
        .collect::<Vec<_>>();

    for (i, index) in indexes.into_iter().enumerate() {
        // TODO: make the error message nicer
        assert!(index > 0);
        assert!(index <= fields.len());

        sorted[index - 1] = Some(fields[i].clone());
    }

    let fields = sorted.into_iter().map(|f| f.unwrap()).collect::<Vec<_>>();

    quote! {
        #[automatically_derived]
        impl #generics ::akroyd::Statement for #name #generics {
            const TEXT: &'static str = #query;

            type Row = #output;

            fn to_row(&self) -> Vec<&(dyn ::akroyd::types::ToSql + Sync)> {
                let mut res = vec![];

                #(
                    res.push(&self.#fields as &(dyn ::akroyd::types::ToSql + Sync));
                )*

                res
            }
        }
    }
}

struct StatementParams {
    text: Option<syn::LitStr>,
    file: Option<syn::LitStr>,
    row: Option<syn::Type>,
}

fn parse_struct_attrs(attrs: &[syn::Attribute]) -> StatementParams {
    let mut params = StatementParams {
        text: None,
        file: None,
        row: None,
    };

    for attr in attrs {
        if attr.path().is_ident("query") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("text") {
                    let value = meta.value()?;
                    params.text = Some(value.parse()?);
                    return Ok(());
                }
                if meta.path.is_ident("file") {
                    let value = meta.value()?;
                    params.file = Some(value.parse()?);
                    return Ok(());
                }
                if meta.path.is_ident("row") {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    params.row = Some(content.parse()?);
                    return Ok(());
                }
                Err(meta.error("unrecognized attribute"))
            })
            .expect("Unable to parse query attribute!");
        }
    }

    params
}

fn parse_field_param_attr(attrs: &[syn::Attribute]) -> Option<usize> {
    let mut index = None;

    for attr in attrs {
        if attr.path().is_ident("query") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("param") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    let text = lit.value();
                    if !text.starts_with('$') {
                        return Err(meta.error("Parameter must be an integer prefixed with $"));
                    }
                    let i: u8 = text.chars().skip(1).collect::<String>().parse().or(Err(
                        meta.error("Parameter must be an integer prefixed with $")
                    ))?;
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

fn parse_field_column_attr(attrs: &[syn::Attribute]) -> Option<syn::Lit> {
    let mut index = None;

    for attr in attrs {
        if attr.path().is_ident("query") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("column") {
                    let value = meta.value()?;
                    let lit: syn::Lit = value.parse()?;
                    index = Some(lit);
                    return Ok(());
                }
                Err(meta.error("unrecognized attribute"))
            })
            .expect("Unable to parse query attribute!");
        }
    }

    index
}
