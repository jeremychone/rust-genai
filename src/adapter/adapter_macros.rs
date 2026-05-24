/// Macro to define adapter kinds and their associated implementations.
///
/// Format:
/// ```ignore
/// /// Comment for the variant
/// @cfg(feature = "feature_name")
/// VariantName => variant_mod [as variant_namespace]
/// ```
///
/// Arguments:
/// `VariantName` is kind name.
/// `variant_mod` is adapter's module name.
/// `variant_namespace` is adapter's namespace. (Optional)
///
/// Optional `variant_namespace`:
/// for cases where the adapter's namespace differs from its mod name (e.g., BedrockApi => bedrock as bedrock_api).
/// if not provided, defaults to the same as `variant_mod`.
///
/// Safety: 
/// `adapter/adapters/${variant_mod}` must exist, contain a struct named `${variant}Adapter`.
/// Adapter must implement the `Adapter` trait.
/// `variant_namespace` should be unique.
/// Use @cfg instead of #[cfg].
macro_rules! define_adapter_kinds {
    (
        $(
            $(#[$meta:meta])*
            $(@cfg(feature = $feature:literal))? // optional cfg for the variant
            $variant:ident => $variant_mod:ident $(as $variant_namespace:ident)?
        ),* $(,)?
    ) => {
        // Fill in the default namespace.
        define_adapter_kinds!(@full $(
            $(#[$meta])*
            $(@cfg(feature = $feature))?
            $variant => $variant_mod $(as $variant_namespace)? as $variant_mod,
        )*);
    };
    // Internal rule.
    (@full $(
            $(#[$meta:meta])*
            $(@cfg(feature = $feature:literal))? // optional cfg for the variant
            $variant:ident => $variant_mod:ident as $variant_namespace:ident $(as $_:ident)?,
    )*) => {


        #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, derive_more::Display, serde::Serialize, serde::Deserialize)]
        pub enum AdapterKind {
            $( 
                $(#[$meta])* 
                $( #[cfg(feature = $feature)] )?
                $variant,
            )*
        }

        define_adapter_kinds!(@impl [
            $({
                variant: $variant,
                mod_name: $variant_mod,
                namespace: $variant_namespace,
                $(feature: $feature,)?
            }),*
        ]);

        define_adapter_kinds!(@macro_rules [
            $({
                variant: $variant,
                mod_name: $variant_mod,
                $(feature: $feature,)?
            }),*
        ]);

    };
    // Implementations
    (@impl [ $({
        variant: $variant:ident,
        mod_name: $mod_name:ident,
        namespace: $namespace:ident,
        $(feature: $feature:literal,)?
    }),* ]) => {

        paste::paste!{
            /// Serialization/Parse implementations
            impl AdapterKind {
                /// Serialize to a static str
                pub fn as_str(&self) -> &'static str {
                    match self {
                        $( 
                            $( #[cfg(feature = $feature)] )?
                            AdapterKind::$variant => stringify!([< $variant >]), 
                        )*
                    }
                }

                /// Serialize to a lowercase static str
                pub fn as_lower_str(&self) -> &'static str {
                    match self {
                        $( 
                            $( #[cfg(feature = $feature)] )?
                            AdapterKind::$variant => stringify!($namespace) 
                        ),*
                    }
                }

                pub fn from_lower_str(name: &str) -> Option<Self> {
                    match name {
                        $( 
                            $( #[cfg(feature = $feature)] )?
                            stringify!($namespace) => Some(AdapterKind::$variant), 
                        )*
                        _ => None,
                    }
                }

                /// Get the default key environment variable name for the adapter kind.
                pub fn default_key_env_name(&self) -> Option<&'static str> {
                    use crate::adapter::Adapter;
                    match self {
                        $( 
                            $( #[cfg(feature = $feature)] )?
                            AdapterKind::$variant => 
                                crate::adapter::$mod_name::[< $variant Adapter >]::DEFAULT_API_KEY_ENV_NAME, )*
                    }
                }
            }
        }

    };

    // Macro for dispatching to the correct adapter implementation based on the adapter kind.
    (@macro_rules [ $({
        variant: $variant:ident,
        mod_name: $mod_name:ident,
        $(feature: $feature:literal,)?
    }),* ]) => {

        paste::paste! {
            /// Dispatch adapter method call based on `AdapterKind`, avoiding repeated `match` arms.
            ///
            /// Usage:
            /// ```ignore
            /// dispatch_adapter!(kind, A::some_method(args))
            /// ```
            ///
            /// Inside the provided expression, a type alias `A` is bound to the appropriate
            /// concrete adapter struct (e.g., `OpenAIAdapter`), allowing static dispatch.
            ///
            /// The macro contains the full mapping from every `AdapterKind` variant to
            /// its corresponding adapter struct, using fully qualified paths.
            macro_rules! dispatch_adapter {
                ($kind:expr, $body:expr) => {
                    match $kind {
                        $( 
                            $( #[cfg(feature = $feature)] )?
                            AdapterKind::$variant => {
                                type A = crate::adapter::$mod_name::[< $variant Adapter >];
                                $body
                            }
                        )*
                    }
                }
            }
        }

        pub (in crate::adapter) use dispatch_adapter;
    };
}

pub (in crate::adapter) use define_adapter_kinds;
