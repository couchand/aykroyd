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

#[proc_macro_derive(Statement, attributes(aykroyd))]
pub fn derive_statement(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let attr = ast.attrs.iter().find(|attr| attr.path().is_ident("aykroyd")).unwrap();

    let fields = match &ast.data {
        syn::Data::Enum(_) => panic!("Cannot derive Statement on enum!"),
        syn::Data::Union(_) => panic!("Cannot derive Statement on union!"),
        syn::Data::Struct(s) => &s.fields,
    };

    let query_text = {
        let mut query_text = None;

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("query") {
                let value = meta.value()?;
                let text: syn::LitStr = value.parse()?;
                query_text = Some(text);
                return Ok(());
            }

            Err(meta.error("unknown meta path"))
        }).unwrap();

        match query_text {
            Some(q) => q,
            None => panic!("unable to find query text"),
        }
    };

    let query_text_impl = impl_static_query_text(name, &query_text);
    let to_params_impl = impl_to_params(name, fields);
    let statement_impl = impl_statement(name);

    let body = quote!(#query_text_impl, #to_params_impl #statement_impl);
    body.into()
}

#[proc_macro_derive(Query, attributes(aykroyd))]
pub fn derive_query(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let attr = ast.attrs.iter().find(|attr| attr.path().is_ident("aykroyd")).unwrap();

    let fields = match &ast.data {
        syn::Data::Enum(_) => panic!("Cannot derive Query on enum!"),
        syn::Data::Union(_) => panic!("Cannot derive Query on union!"),
        syn::Data::Struct(s) => &s.fields,
    };

    let (query_text, row) = {
        let mut query_text = None;
        let mut row = None;

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("query") {
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
                return Ok(())
            }

            Err(meta.error("unknown meta path"))
        }).unwrap();

        match (query_text, row) {
            (Some(q), Some(r)) => (q, r),
            (None, _) => panic!("unable to find query text"),
            (_, None) => panic!("unable to find row"),
        }
    };

    let query_text_impl = impl_static_query_text(name, &query_text);
    let to_params_impl = impl_to_params(name, fields);
    let query_impl = impl_query(name, &row);

    let body = quote!(#query_text_impl #to_params_impl #query_impl);
    body.into()
}

fn impl_static_query_text(name: &syn::Ident, query_text: &syn::LitStr) -> proc_macro2::TokenStream {
    quote! {
        #[automatically_derived]
        impl ::aykroyd_v2::query::StaticQueryText for #name {
            const QUERY_TEXT: &'static str = #query_text;
        }
    }
}

fn impl_to_params(name: &syn::Ident, fields: &syn::Fields) -> proc_macro2::TokenStream {
    let fields = match &fields {
        syn::Fields::Unit => vec![],
        syn::Fields::Named(syn::FieldsNamed { named: fields, .. }) |
            syn::Fields::Unnamed(syn::FieldsUnnamed { unnamed: fields, .. }) => {
            fields.into_iter().collect()
        }
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
            ::aykroyd_v2::client::ToParam::to_param(&self.#name)
        });

        let ty = &field.ty;
        wheres.push(quote! {
            #ty: ::aykroyd_v2::client::ToParam<C>
        });
    }
    quote! {
        #[automatically_derived]
        impl<C> ::aykroyd_v2::query::ToParams<C> for #name
        where
            C: ::aykroyd_v2::client::Client,
            #(#wheres,)*
        {
            fn to_params(&self) -> Vec<<C as ::aykroyd_v2::client::Client>::Param<'_>> {
                [
                    #(#params,)*
                ].into()
            }
        }
    }
}

fn impl_statement(name: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        #[automatically_derived]
        impl<C> ::aykroyd_v2::query::Statement<C> for #name
        where
            C: ::aykroyd_v2::client::Client,
            Self: ::aykroyd_v2::query::ToParams<C>,
        {
        }
    }
}

fn impl_query(name: &syn::Ident, row: &syn::Type) -> proc_macro2::TokenStream {
    quote! {
        #[automatically_derived]
        impl<C> ::aykroyd_v2::query::Query<C> for #name
        where
            C: ::aykroyd_v2::client::Client,
            #row: ::aykroyd_v2::row::FromRow<C>,
            Self: ::aykroyd_v2::query::ToParams<C>,
        {
            type Row = #row;
        }
    }
}

#[proc_macro_derive(FromRow, attributes(aykroyd))]
pub fn derive_from_row(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let fields = match &ast.data {
        syn::Data::Enum(_) => panic!("Cannot derive FromRow on enum!"),
        syn::Data::Union(_) => panic!("Cannot derive FromRow on union!"),
        syn::Data::Struct(s) => &s.fields,
    };
    let tuple_struct = match &fields {
        syn::Fields::Unit | syn::Fields::Unnamed(_) => true,
        syn::Fields::Named(_) => false,
    };
    let fields = match &fields {
        syn::Fields::Unit => vec![],
        syn::Fields::Named(syn::FieldsNamed { named: fields, .. }) |
            syn::Fields::Unnamed(syn::FieldsUnnamed { unnamed: fields, .. }) => {
            fields.into_iter().collect()
        }
    };

    let mut key = None;

    if let Some(attr) = ast.attrs.iter().find(|attr| attr.path().is_ident("aykroyd")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("by_index") {
                key = Some(Key::Index);
                return Ok(());
            }

            if meta.path.is_ident("by_name") {
                key = Some(Key::Name);
                return Ok(())
            }

            Err(meta.error("unknown meta path"))
        }).unwrap();
    }

    let key = key.unwrap_or_else(|| if tuple_struct { Key::Index } else { Key::Name });

    let from_columns_impl = impl_from_columns(key, name, tuple_struct, &fields[..]);
    let from_row_impl = impl_from_row(key, name);

    let body = quote!(#from_row_impl #from_columns_impl);
    body.into()
}

#[proc_macro_derive(FromColumnsIndexed, attributes(aykroyd))]
pub fn derive_from_columns_indexed(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let fields = match &ast.data {
        syn::Data::Enum(_) => panic!("Cannot derive FromColumnsIndexed on enum!"),
        syn::Data::Union(_) => panic!("Cannot derive FromColumnsIndexed on union!"),
        syn::Data::Struct(s) => &s.fields,
    };
    let tuple_struct = match &fields {
        syn::Fields::Unit | syn::Fields::Unnamed(_) => true,
        syn::Fields::Named(_) => false,
    };
    let fields = match &fields {
        syn::Fields::Unit => vec![],
        syn::Fields::Named(syn::FieldsNamed { named: fields, .. }) |
            syn::Fields::Unnamed(syn::FieldsUnnamed { unnamed: fields, .. }) => {
            fields.into_iter().collect()
        }
    };

    let body = impl_from_columns(Key::Index, name, tuple_struct, &fields[..]);
    body.into()
}

#[proc_macro_derive(FromColumnsNamed, attributes(aykroyd))]
pub fn derive_from_columns_named(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let fields = match &ast.data {
        syn::Data::Enum(_) => panic!("Cannot derive FromColumnsNamed on enum!"),
        syn::Data::Union(_) => panic!("Cannot derive FromColumnsNamed on union!"),
        syn::Data::Struct(s) => &s.fields,
    };
    let tuple_struct = match &fields {
        syn::Fields::Unit | syn::Fields::Unnamed(_) => true,
        syn::Fields::Named(_) => false,
    };
    let fields = match &fields {
        syn::Fields::Unit => vec![],
        syn::Fields::Named(syn::FieldsNamed { named: fields, .. }) |
            syn::Fields::Unnamed(syn::FieldsUnnamed { unnamed: fields, .. }) => {
            fields.into_iter().collect()
        }
    };

    let body = impl_from_columns(Key::Name, name, tuple_struct, &fields[..]);
    body.into()
}

fn select_from_columns_delegate(attrs: &[syn::Attribute]) -> Delegate {
    for attr in attrs {
        if attr.path().is_ident("aykroyd") {
            let mut delegate = None;
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("nested") {
                    delegate = Some(Delegate::FromColumns);
                }

                // TODO: centralize parsing!
                Ok(())
            }).unwrap();
            if let Some(delegate) = delegate {
                return delegate;
            }
        }
    }

    Delegate::FromColumn
}

fn impl_from_row(
    key: Key,
    name: &syn::Ident,
) -> proc_macro2::TokenStream {
    let (trait_ty, column_ty) = match key {
        Key::Index => (quote!(FromColumnsIndexed), quote!(ColumnsIndexed)),
        Key::Name => (quote!(FromColumnsNamed), quote!(ColumnsNamed)),
    };

    quote! {
        #[automatically_derived]
        impl<C> ::aykroyd_v2::row::FromRow<C> for #name
        where
            C: ::aykroyd_v2::client::Client,
            Self: ::aykroyd_v2::row::#trait_ty<C>,
        {
            fn from_row(
                row: &C::Row<'_>,
            ) -> Result<Self, ::aykroyd_v2::error::Error<C::Error>> {
                ::aykroyd_v2::row::#trait_ty::from_columns(
                    ::aykroyd_v2::row::#column_ty::new(row),
                )
            }
        }
    }
}

fn impl_from_columns(
    key: Key,
    name: &syn::Ident,
    tuple_struct: bool,
    fields: &[&syn::Field],
) -> proc_macro2::TokenStream {
    let mut wheres = vec![];
    let mut num_const = 0;
    let mut plus_nesteds = vec![];
    let mut field_puts = vec![];
    for (index, field) in fields.iter().enumerate() {
        let ty = &field.ty;
        let delegate = select_from_columns_delegate(&field.attrs);

        {
            use Key::*;
            use Delegate::*;
            let delegate = match (key, delegate) {
                (Index, FromColumn) => quote!(::aykroyd_v2::client::FromColumnIndexed),
                (Index, FromColumns) => quote!(::aykroyd_v2::row::FromColumnsIndexed),
                (Name, FromColumn) => quote!(::aykroyd_v2::client::FromColumnNamed),
                (Name, FromColumns) => quote!(::aykroyd_v2::row::FromColumnsNamed),
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
                    // TODO: explicit index
                    let num_const = syn::LitInt::new(
                        &format!("{num_const}usize"),
                        proc_macro2::Span::call_site(),
                    );
                    quote!(#num_const #(#plus_nesteds)*)
                }
                Key::Name => {
                    // TODO: explicit name
                    let name = field.ident
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| index.to_string());

                    let name = match delegate {
                        Delegate::FromColumn => name,
                        Delegate::FromColumns => {
                            // TODO: explicit name as prefix
                            let mut s = name;
                            s.push('_');
                            s
                        }
                    };
                    quote!(#name)
                }
            };
            field_puts.push(match &field.ident {
                Some(field_name) => quote!(#field_name: columns.#get_method(#key)?),
                None => quote!(columns.#get_method(#key)?),
            });
        }

        match delegate {
            Delegate::FromColumn => num_const += 1,
            Delegate::FromColumns => plus_nesteds.push(quote!(+ #ty::NUM_COLUMNS)),
        }

    }

    let field_list = if !tuple_struct {
        quote!({#(#field_puts),*})
    } else {
        quote!((#(#field_puts),*))
    };
    let num_const = syn::LitInt::new(
        &format!("{num_const}usize"),
        proc_macro2::Span::call_site(),
    );

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
        impl<C> ::aykroyd_v2::row::#trait_ty<C> for #name
        where
            C: ::aykroyd_v2::client::Client,
            #(#wheres),*
        {
            #num_columns

            fn from_columns(
                columns: ::aykroyd_v2::row::#column_ty<C>,
            ) -> Result<Self, ::aykroyd_v2::error::Error<C::Error>> {
                Ok(#name #field_list)
            }
        }
    }
}
