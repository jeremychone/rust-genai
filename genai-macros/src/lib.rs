use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Field, Attribute, Meta, Expr, Lit};

/// Derive macro for generating GenAI tool schemas
#[proc_macro_derive(GenAiTool, attributes(tool))]
pub fn derive_genai_tool(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    match generate_genai_tool(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn generate_genai_tool(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    
    // Parse tool attributes
    let tool_attrs = parse_tool_attrs(&input.attrs)?;
    let tool_name = tool_attrs.name.unwrap_or_else(|| name.to_string().to_lowercase());
    let tool_description = tool_attrs.description;
    
    // Generate schema based on struct fields
    let schema = generate_schema(&input.data)?;
    
    let tool_description_expr = match tool_description {
        Some(desc) => quote! { Some(#desc) },
        None => quote! { None },
    };
    
    let expanded = quote! {
        impl ::genai::chat::tool::GenAiTool for #name {
            fn tool_name() -> &'static str {
                #tool_name
            }
            
            fn tool_description() -> Option<&'static str> {
                #tool_description_expr
            }
            
            fn json_schema() -> ::serde_json::Value {
                #schema
            }
            
            fn from_tool_call(tool_call: &::genai::chat::ToolCall) -> ::std::result::Result<Self, ::genai::chat::tool::ToolError> {
                ::serde_json::from_value(tool_call.fn_arguments.clone())
                                         .map_err(::genai::chat::tool::ToolError::DeserializationError)
            }
        }
        
        impl ::std::convert::From<#name> for ::genai::chat::Tool {
            fn from(_: #name) -> Self {
                ::genai::chat::Tool::new(#name::tool_name())
                    .with_description(#name::tool_description().unwrap_or(""))
                    .with_schema(#name::json_schema())
            }
        }
        
        // Note: We don't implement Default automatically as it's better for users to derive it themselves
        // or implement it manually with proper field defaults
    };
    
    Ok(expanded)
}

#[derive(Default)]
struct ToolAttrs {
    name: Option<String>,
    description: Option<String>,
}

fn parse_tool_attrs(attrs: &[Attribute]) -> syn::Result<ToolAttrs> {
    let mut tool_attrs = ToolAttrs::default();
    
    for attr in attrs {
        if attr.path().is_ident("tool") {
            match &attr.meta {
                Meta::List(meta_list) => {
                    // Parse the meta list for name=value pairs
                    let nested = meta_list.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated);
                    if let Ok(nested_metas) = nested {
                        for nested_meta in nested_metas {
                            match nested_meta {
                                syn::Meta::NameValue(nv) if nv.path.is_ident("name") => {
                                    if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &nv.value {
                                        tool_attrs.name = Some(s.value());
                                    }
                                }
                                syn::Meta::NameValue(nv) if nv.path.is_ident("description") => {
                                    if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &nv.value {
                                        tool_attrs.description = Some(s.value());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    
    Ok(tool_attrs)
}

fn generate_schema(data: &Data) -> syn::Result<proc_macro2::TokenStream> {
    match data {
        Data::Struct(data_struct) => {
            match &data_struct.fields {
                Fields::Named(fields) => {
                    let mut properties = Vec::new();
                    let mut required = Vec::new();
                    
                    for field in &fields.named {
                        let field_name = field.ident.as_ref().unwrap().to_string();
                        let field_schema = generate_field_schema(field)?;
                        
                        properties.push(quote! {
                            (#field_name.to_string(), #field_schema)
                        });
                        
                        // For now, all fields are required (we'll enhance this later)
                        if !is_option_type(&field.ty) {
                            required.push(field_name);
                        }
                    }
                    
                    Ok(quote! {
                        ::serde_json::json!({
                            "type": "object",
                            "properties": ::serde_json::Map::from_iter([
                                #(#properties),*
                            ]),
                            "required": [#(#required),*]
                        })
                    })
                }
                _ => Err(syn::Error::new_spanned(&data_struct.fields, "Only named fields are supported")),
            }
        }
        _ => {
            let span = proc_macro2::Span::call_site();
            Err(syn::Error::new(span, "Only structs are supported"))
        },
    }
}

fn generate_field_schema(field: &Field) -> syn::Result<proc_macro2::TokenStream> {
    let field_doc = extract_doc_comment(&field.attrs);
    let field_attrs = parse_field_attrs(&field.attrs)?;
    
    // Determine the JSON schema type based on Rust type
    let (json_type, extra_props) = determine_json_type(&field.ty, &field_attrs)?;
    
    let description = field_attrs.description.or(field_doc);
    
    let mut schema_props = vec![
        quote! { ("type".to_string(), ::serde_json::Value::String(#json_type.to_string())) }
    ];
    
    if let Some(desc) = description {
        schema_props.push(quote! {
            ("description".to_string(), ::serde_json::Value::String(#desc.to_string()))
        });
    }
    
    // Add enum values if specified
    if let Some(enum_values) = field_attrs.enum_values {
        let enum_vals: Vec<_> = enum_values.iter().map(|v| quote! { #v }).collect();
        schema_props.push(quote! {
            ("enum".to_string(), ::serde_json::Value::Array(vec![#(::serde_json::Value::String(#enum_vals.to_string())),*]))
        });
    }
    
    // Add extra properties
    schema_props.extend(extra_props);
    
    Ok(quote! {
        ::serde_json::Value::Object(::serde_json::Map::from_iter([
            #(#schema_props),*
        ]))
    })
}

#[derive(Default)]
struct FieldAttrs {
    description: Option<String>,
    enum_values: Option<Vec<String>>,
    default: Option<String>,
}

fn parse_field_attrs(attrs: &[Attribute]) -> syn::Result<FieldAttrs> {
    let mut field_attrs = FieldAttrs::default();
    
    for attr in attrs {
        if attr.path().is_ident("tool") {
            // Simplified parsing - you'd want more robust parsing in production
            let tokens_str = match &attr.meta {
                Meta::List(meta_list) => meta_list.tokens.to_string(),
                _ => continue,
            };
            
            if tokens_str.contains("enum_values") {
                // Extract enum values - this is a very basic parser
                if let Some(start) = tokens_str.find('[') {
                    if let Some(end) = tokens_str.find(']') {
                        let values_str = &tokens_str[start+1..end];
                        let values: Vec<String> = values_str
                            .split(',')
                            .map(|s| s.trim().trim_matches('"').to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        field_attrs.enum_values = Some(values);
                    }
                }
            }
        }
    }
    
    Ok(field_attrs)
}

fn extract_doc_comment(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(meta) = &attr.meta {
                if let Expr::Lit(expr_lit) = &meta.value {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        return Some(lit_str.value().trim().to_string());
                    }
                }
            }
        }
    }
    None
}

fn determine_json_type(ty: &syn::Type, _attrs: &FieldAttrs) -> syn::Result<(&'static str, Vec<proc_macro2::TokenStream>)> {
    // Convert Rust types to JSON schema types
    let type_str = quote! { #ty }.to_string();
    
    let json_type = match type_str.as_str() {
        "String" | "&str" => "string",
        "i32" | "i64" | "u32" | "u64" | "isize" | "usize" => "integer", 
        "f32" | "f64" => "number",
        "bool" => "boolean",
        _ if type_str.starts_with("Option <") => {
            // For Option<T>, determine the inner type
            // This is simplified - you'd want proper type parsing
            if type_str.contains("String") {
                "string"
            } else if type_str.contains("i32") || type_str.contains("u32") {
                "integer"
            } else if type_str.contains("f64") || type_str.contains("f32") {
                "number"
            } else if type_str.contains("bool") {
                "boolean"
            } else {
                "string" // Default fallback
            }
        },
        _ => "string", // Default fallback
    };
    
    Ok((json_type, vec![]))
}

fn is_option_type(ty: &syn::Type) -> bool {
    let type_str = quote! { #ty }.to_string();
    type_str.starts_with("Option <")
} 