use quote::quote;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Index,
    Name,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Delegate {
    FromColumn,
    FromColumns,
}

/// Derive macro available if aykroyd is built with `features = ["derive"]`.
#[proc_macro_derive(Statement, attributes(aykroyd))]
pub fn derive_statement(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let generics = &ast.generics;
    let attr = ast
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("aykroyd"))
        .unwrap();

    let fields = match &ast.data {
        syn::Data::Enum(_) => panic!("Cannot derive Statement on enum!"),
        syn::Data::Union(_) => panic!("Cannot derive Statement on union!"),
        syn::Data::Struct(s) => &s.fields,
    };

    let query_text = {
        let mut query_text = None;

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("text") {
                let value = meta.value()?;
                let text: syn::LitStr = value.parse()?;
                query_text = Some(text);
                return Ok(());
            }

            Err(meta.error("unknown meta path"))
        })
        .unwrap();

        match query_text {
            Some(q) => q,
            None => panic!("unable to find query text"),
        }
    };

    let query_text_impl = impl_static_query_text(name, generics, &query_text);
    let to_params_impl = impl_to_params(name, generics, fields);
    let statement_impl = impl_statement(name, generics);

    let body = quote!(#query_text_impl #to_params_impl #statement_impl);
    body.into()
}

/// Derive macro available if aykroyd is built with `features = ["derive"]`.
#[proc_macro_derive(Query, attributes(aykroyd))]
pub fn derive_query(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let generics = &ast.generics;
    let attr = ast
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("aykroyd"))
        .unwrap();

    let fields = match &ast.data {
        syn::Data::Enum(_) => panic!("Cannot derive Query on enum!"),
        syn::Data::Union(_) => panic!("Cannot derive Query on union!"),
        syn::Data::Struct(s) => &s.fields,
    };

    let (query_text, row) = {
        let mut query_text = None;
        let mut row = None;

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("text") {
                let value = meta.value()?;
                let text: syn::LitStr = value.parse()?;
                query_text = Some(text);
                return Ok(());
            }

            if meta.path.is_ident("row") {
                let content;
                syn::parenthesized!(content in meta.input);
                let ty: syn::Type = content.parse()?;
                row = Some(ty);
                return Ok(());
            }

            Err(meta.error("unknown meta path"))
        })
        .unwrap();

        match (query_text, row) {
            (Some(q), Some(r)) => (q, r),
            (None, _) => panic!("unable to find query text"),
            (_, None) => panic!("unable to find row"),
        }
    };

    let query_text_impl = impl_static_query_text(name, generics, &query_text);
    let to_params_impl = impl_to_params(name, generics, fields);
    let query_impl = impl_query(name, generics, &row);

    let body = quote!(#query_text_impl #to_params_impl #query_impl);
    body.into()
}

/// Derive macro available if aykroyd is built with `features = ["derive"]`.
#[proc_macro_derive(QueryOne, attributes(aykroyd))]
pub fn derive_query_one(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let generics = &ast.generics;
    let attr = ast
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("aykroyd"))
        .unwrap();

    let fields = match &ast.data {
        syn::Data::Enum(_) => panic!("Cannot derive QueryOne on enum!"),
        syn::Data::Union(_) => panic!("Cannot derive QueryOne on union!"),
        syn::Data::Struct(s) => &s.fields,
    };

    let (query_text, row) = {
        let mut query_text = None;
        let mut row = None;

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("text") {
                let value = meta.value()?;
                let text: syn::LitStr = value.parse()?;
                query_text = Some(text);
                return Ok(());
            }

            if meta.path.is_ident("row") {
                let content;
                syn::parenthesized!(content in meta.input);
                let ty: syn::Type = content.parse()?;
                row = Some(ty);
                return Ok(());
            }

            Err(meta.error("unknown meta path"))
        })
        .unwrap();

        match (query_text, row) {
            (Some(q), Some(r)) => (q, r),
            (None, _) => panic!("unable to find query text"),
            (_, None) => panic!("unable to find row"),
        }
    };

    let query_text_impl = impl_static_query_text(name, generics, &query_text);
    let to_params_impl = impl_to_params(name, generics, fields);
    let query_impl = impl_query(name, generics, &row);
    let query_one_impl = impl_query_one(name, generics);

    let body = quote!(#query_text_impl #to_params_impl #query_impl #query_one_impl);
    body.into()
}

fn simplify(generics: &syn::Generics) -> proc_macro2::TokenStream {
    let params = generics.params.iter().map(|param| {
        use syn::GenericParam::*;
        match param {
            Lifetime(syn::LifetimeParam { lifetime, .. }) => quote!(#lifetime),
            Type(syn::TypeParam { ident, .. }) => quote!(#ident),
            Const(syn::ConstParam { ident, .. }) => quote!(#ident),
        }
    });

    quote!(<#(#params)*>)
}

fn insert_c(generics: &syn::Generics) -> syn::Generics {
    let param = syn::TypeParam {
        attrs: vec![],
        ident: syn::Ident::new("C", proc_macro2::Span::call_site()),
        colon_token: None,
        bounds: syn::punctuated::Punctuated::new(),
        eq_token: None,
        default: None,
    };

    let mut generics = generics.clone();
    generics.params.insert(0, syn::GenericParam::Type(param));
    generics
}

fn impl_static_query_text(
    name: &syn::Ident,
    generics: &syn::Generics,
    query_text: &syn::LitStr,
) -> proc_macro2::TokenStream {
    let generics_simple = simplify(generics);
    let query_text = query_text.value();
    let query_text = query_text.trim();
    quote! {
        #[automatically_derived]
        impl #generics ::aykroyd::query::StaticQueryText for #name #generics_simple {
            const QUERY_TEXT: &'static str = #query_text;
        }
    }
}

fn impl_to_params(
    name: &syn::Ident,
    generics: &syn::Generics,
    fields: &syn::Fields,
) -> proc_macro2::TokenStream {
    let fields = match &fields {
        syn::Fields::Unit => vec![],
        syn::Fields::Named(syn::FieldsNamed { named: fields, .. })
        | syn::Fields::Unnamed(syn::FieldsUnnamed {
            unnamed: fields, ..
        }) => fields.into_iter().collect(),
    };

    let mut params = vec![];
    let mut wheres = vec![];

    for (index, field) in fields.iter().enumerate() {
        let name = match &field.ident {
            Some(name) => quote!(#name),
            None => {
                let index = index as u32;
                let span = proc_macro2::Span::call_site();
                let index = syn::Index { index, span };
                quote!(#index)
            }
        };
        params.push(quote! {
            ::aykroyd::client::ToParam::to_param(&self.#name)
        });

        let ty = &field.ty;
        wheres.push(quote! {
            #ty: ::aykroyd::client::ToParam<C>
        });
    }

    let body = if params.is_empty() {
        quote!(None)
    } else {
        quote!(Some(vec![#(#params,)*]))
    };

    let generics_simple = simplify(generics);
    let generics = insert_c(generics);
    quote! {
        #[automatically_derived]
        impl #generics ::aykroyd::query::ToParams<C> for #name #generics_simple
        where
            C: ::aykroyd::client::Client,
            #(#wheres,)*
        {
            fn to_params(&self) -> Option<Vec<<C as ::aykroyd::client::Client>::Param<'_>>> {
                #body
            }
        }
    }
}

fn impl_statement(name: &syn::Ident, generics: &syn::Generics) -> proc_macro2::TokenStream {
    let generics_simple = simplify(generics);
    let generics = insert_c(generics);
    quote! {
        #[automatically_derived]
        impl #generics ::aykroyd::Statement<C> for #name #generics_simple
        where
            C: ::aykroyd::client::Client,
            Self: ::aykroyd::query::ToParams<C>,
        {
        }
    }
}

fn impl_query(
    name: &syn::Ident,
    generics: &syn::Generics,
    row: &syn::Type,
) -> proc_macro2::TokenStream {
    let generics_simple = simplify(generics);
    let generics = insert_c(generics);
    quote! {
        #[automatically_derived]
        impl #generics ::aykroyd::Query<C> for #name #generics_simple
        where
            C: ::aykroyd::client::Client,
            #row: ::aykroyd::FromRow<C>,
            Self: ::aykroyd::query::ToParams<C>,
        {
            type Row = #row;
        }
    }
}

fn impl_query_one(
    name: &syn::Ident,
    generics: &syn::Generics,
) -> proc_macro2::TokenStream {
    let generics_simple = simplify(generics);
    let generics = insert_c(generics);
    quote! {
        #[automatically_derived]
        impl #generics ::aykroyd::QueryOne<C> for #name #generics_simple
        where
            C: ::aykroyd::client::Client,
            Self: ::aykroyd::Query<C>,
        {
        }
    }
}

/// Derive macro available if aykroyd is built with `features = ["derive"]`.
#[proc_macro_derive(FromRow, attributes(aykroyd))]
pub fn derive_from_row(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let fields = match &ast.data {
        syn::Data::Enum(_) => panic!("Cannot derive FromRow on enum!"),
        syn::Data::Union(_) => panic!("Cannot derive FromRow on union!"),
        syn::Data::Struct(s) => &s.fields,
    };
    let tuple_struct = match fields {
        syn::Fields::Unit | syn::Fields::Unnamed(_) => true,
        syn::Fields::Named(_) => false,
    };
    let fields = match fields {
        syn::Fields::Unit => vec![],
        syn::Fields::Named(syn::FieldsNamed { named: fields, .. })
        | syn::Fields::Unnamed(syn::FieldsUnnamed {
            unnamed: fields, ..
        }) => fields.iter().collect(),
    };
    let fields = FieldInfo::from_fields(&fields);

    let mut key = None;

    if let Some(attr) = ast
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("aykroyd"))
    {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("by_index") {
                key = Some(Key::Index);
                return Ok(());
            }

            if meta.path.is_ident("by_name") {
                key = Some(Key::Name);
                return Ok(());
            }

            Err(meta.error("unknown meta path"))
        })
        .unwrap();
    }

    let key = match FieldInfo::key_for(key, &fields) {
        Err(message) => return message.into(),
        Ok(key) => key,
    };
    let key = key.unwrap_or(if tuple_struct { Key::Index } else { Key::Name });

    let from_columns_impl = impl_from_columns(key, name, tuple_struct, &fields[..]);
    let from_row_impl = impl_from_row(key, name);

    let body = quote!(#from_row_impl #from_columns_impl);
    body.into()
}

/// Derive macro available if aykroyd is built with `features = ["derive"]`.
#[proc_macro_derive(FromColumnsIndexed, attributes(aykroyd))]
pub fn derive_from_columns_indexed(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let fields = match &ast.data {
        syn::Data::Enum(_) => panic!("Cannot derive FromColumnsIndexed on enum!"),
        syn::Data::Union(_) => panic!("Cannot derive FromColumnsIndexed on union!"),
        syn::Data::Struct(s) => &s.fields,
    };
    let tuple_struct = match fields {
        syn::Fields::Unit | syn::Fields::Unnamed(_) => true,
        syn::Fields::Named(_) => false,
    };
    let fields = match fields {
        syn::Fields::Unit => vec![],
        syn::Fields::Named(syn::FieldsNamed { named: fields, .. })
        | syn::Fields::Unnamed(syn::FieldsUnnamed {
            unnamed: fields, ..
        }) => fields.iter().collect(),
    };
    let fields = FieldInfo::from_fields(&fields);
    if let Err(message) = FieldInfo::assert_key(Key::Index, &fields) {
        return message.into();
    }

    let body = impl_from_columns(Key::Index, name, tuple_struct, &fields[..]);
    body.into()
}

/// Derive macro available if aykroyd is built with `features = ["derive"]`.
#[proc_macro_derive(FromColumnsNamed, attributes(aykroyd))]
pub fn derive_from_columns_named(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let fields = match &ast.data {
        syn::Data::Enum(_) => panic!("Cannot derive FromColumnsNamed on enum!"),
        syn::Data::Union(_) => panic!("Cannot derive FromColumnsNamed on union!"),
        syn::Data::Struct(s) => &s.fields,
    };
    let tuple_struct = match fields {
        syn::Fields::Unit | syn::Fields::Unnamed(_) => true,
        syn::Fields::Named(_) => false,
    };
    let fields = match fields {
        syn::Fields::Unit => vec![],
        syn::Fields::Named(syn::FieldsNamed { named: fields, .. })
        | syn::Fields::Unnamed(syn::FieldsUnnamed {
            unnamed: fields, ..
        }) => fields.iter().collect(),
    };
    let fields = FieldInfo::from_fields(&fields);
    if let Err(message) = FieldInfo::assert_key(Key::Index, &fields) {
        return message.into();
    }

    let body = impl_from_columns(Key::Name, name, tuple_struct, &fields[..]);
    body.into()
}

struct FieldInfo {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    nested: bool,
    column: Option<syn::Lit>,
}

impl FieldInfo {
    fn from_fields(fields: &[&syn::Field]) -> Vec<FieldInfo> {
        fields.iter().map(|field| {
            let ident = field.ident.clone();
            let ty = field.ty.clone();
            let mut nested = false;
            let mut column = None;

            for attr in &field.attrs {
                if attr.path().is_ident("aykroyd") {
                    attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("nested") {
                            nested = true;
                            return Ok(());
                        }

                        if meta.path.is_ident("column") {
                            let value = meta.value()?;
                            let inner = value.parse()?;
                            column = Some(inner);
                            return Ok(());
                        }

                        Err(meta.error("unrecognized attr"))
                    })
                    .unwrap();
                }
            }

            FieldInfo { ident, ty, nested, column }
        }).collect()
    }

    fn assert_key(
        expected: Key,
        fields: &[FieldInfo],
    ) -> Result<Option<Key>, proc_macro2::TokenStream> {
        FieldInfo::key_for(Some(expected), fields)
    }

    fn key_for(
        expected: Option<Key>,
        fields: &[FieldInfo],
    ) -> Result<Option<Key>, proc_macro2::TokenStream> {
        let key = fields
            .iter()
            .find_map(|field| field.column.as_ref())
            .map(|lit| {
                match lit {
                    syn::Lit::Int(_) => Ok(Key::Index),
                    syn::Lit::Str(_) => Ok(Key::Name),
                    _ => {
                        Err(quote::quote_spanned! {
                            lit.span() => compile_error!("invalid column key");
                        })
                    }
                }
            })
            .transpose()?;

        if let Some(key) = key {
            let key = expected.unwrap_or(key);
            for field in fields {
                match key {
                    Key::Index => {
                        match &field.column {
                            Some(syn::Lit::Int(_)) => {}
                            Some(lit) => {
                                return Err(quote::quote_spanned! {
                                    lit.span() => compile_error!("expected column index");
                                });
                            }
                            None => {
                                use syn::spanned::Spanned;
                                return Err(quote::quote_spanned! {
                                    field.ty.span() => compile_error!("expected column index");
                                });
                            }
                        }
                    }
                    Key::Name => {
                        match &field.column {
                            Some(syn::Lit::Str(_)) => {}
                            Some(lit) => {
                                return Err(quote::quote_spanned! {
                                    lit.span() => compile_error!("expected column name");
                                });
                            }
                            None => {
                                use syn::spanned::Spanned;
                                return Err(quote::quote_spanned! {
                                    field.ty.span() => compile_error!("expected column name");
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(expected.or(key))
    }
}

fn impl_from_row(key: Key, name: &syn::Ident) -> proc_macro2::TokenStream {
    let (trait_ty, column_ty) = match key {
        Key::Index => (quote!(FromColumnsIndexed), quote!(ColumnsIndexed)),
        Key::Name => (quote!(FromColumnsNamed), quote!(ColumnsNamed)),
    };

    quote! {
        #[automatically_derived]
        impl<C> ::aykroyd::FromRow<C> for #name
        where
            C: ::aykroyd::client::Client,
            Self: ::aykroyd::row::#trait_ty<C>,
        {
            fn from_row(
                row: &C::Row<'_>,
            ) -> Result<Self, ::aykroyd::error::Error<C::Error>> {
                ::aykroyd::row::#trait_ty::from_columns(
                    ::aykroyd::row::#column_ty::new(row),
                )
            }
        }
    }
}

fn impl_from_columns(
    key: Key,
    name: &syn::Ident,
    tuple_struct: bool,
    fields: &[FieldInfo],
) -> proc_macro2::TokenStream {
    let mut wheres = vec![];
    let mut num_const = 0;
    let mut plus_nesteds = vec![];
    let mut field_puts = vec![];
    for (index, field) in fields.iter().enumerate() {
        let ty = &field.ty;
        let delegate = if field.nested {
            Delegate::FromColumns
        } else {
            Delegate::FromColumn
        };

        {
            use Delegate::*;
            use Key::*;
            let delegate = match (key, delegate) {
                (Index, FromColumn) => quote!(::aykroyd::client::FromColumnIndexed),
                (Index, FromColumns) => quote!(::aykroyd::row::FromColumnsIndexed),
                (Name, FromColumn) => quote!(::aykroyd::client::FromColumnNamed),
                (Name, FromColumns) => quote!(::aykroyd::row::FromColumnsNamed),
            };
            wheres.push(quote!(#ty: #delegate<C>));
        }

        {
            let get_method = match delegate {
                Delegate::FromColumn => quote!(get),
                Delegate::FromColumns => quote!(get_nested),
            };
            let key = match key {
                Key::Index => {
                    match &field.column {
                        Some(index) => {
                            quote!(#index)
                        }
                        None => {
                            let num_const = syn::LitInt::new(
                                &format!("{num_const}usize"),
                                proc_macro2::Span::call_site(),
                            );
                            quote!(#num_const #(#plus_nesteds)*)
                        }
                    }
                }
                Key::Name => {
                    match &field.column {
                        Some(name) => {
                            quote!(#name)
                        }
                        None => {
                            let name = field
                                .ident
                                .as_ref()
                                .map(ToString::to_string)
                                .unwrap_or_else(|| index.to_string());

                            let name = match delegate {
                                Delegate::FromColumn => name,
                                Delegate::FromColumns => {
                                    let mut s = name;
                                    s.push('_');
                                    s
                                }
                            };
                            quote!(#name)
                        }
                    }
                }
            };
            field_puts.push(match &field.ident {
                Some(field_name) => quote!(#field_name: columns.#get_method(#key)?),
                None => quote!(columns.#get_method(#key)?),
            });
        }

        if let Some(lit) = &field.column {
            if let syn::Lit::Int(index) = lit {
                let index: usize = index.base10_parse().unwrap();
                num_const = index;
                plus_nesteds.clear();
            }
        }

        match delegate {
            Delegate::FromColumn => num_const += 1,
            Delegate::FromColumns => plus_nesteds.push(quote!(+ <#ty as ::aykroyd::row::FromColumnsIndexed<C>>::NUM_COLUMNS)),
        }
    }

    let field_list = if !tuple_struct {
        quote!({#(#field_puts),*})
    } else if !field_puts.is_empty() {
        quote!((#(#field_puts),*))
    } else {
        quote!()
    };
    let num_const = syn::LitInt::new(&format!("{num_const}usize"), proc_macro2::Span::call_site());

    let (trait_ty, column_ty) = match key {
        Key::Index => (quote!(FromColumnsIndexed), quote!(ColumnsIndexed)),
        Key::Name => (quote!(FromColumnsNamed), quote!(ColumnsNamed)),
    };

    let num_columns = match key {
        Key::Index => quote!(const NUM_COLUMNS: usize = #num_const #(#plus_nesteds)*;),
        Key::Name => quote!(),
    };

    quote! {
        #[automatically_derived]
        impl<C> ::aykroyd::row::#trait_ty<C> for #name
        where
            C: ::aykroyd::client::Client,
            #(#wheres),*
        {
            #num_columns

            fn from_columns(
                columns: ::aykroyd::row::#column_ty<C>,
            ) -> Result<Self, ::aykroyd::error::Error<C::Error>> {
                Ok(#name #field_list)
            }
        }
    }
}
