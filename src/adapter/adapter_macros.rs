macro_rules! define_adapter_kinds {
    (
        $(
            $(#[$meta:meta])*
            $(@cfg(feature = $feature:literal))? // optional cfg for the variant
            $variant:ident => $variant_mod:ident $(as $variant_namespace:ident)?
        ),* $(,)?
    ) => {
        define_adapter_kinds!(@full $(
            $(#[$meta])*
            $(@cfg(feature = $feature))?
            $variant => $variant_mod $(as $variant_namespace)? as $variant_mod,
        )*);
    };
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
    (@impl [ $({
        // camel name
        variant: $variant:ident,
        // mod name
        mod_name: $mod_name:ident,
        // lower str, because some adapter's namespace is different from their mod name,
        namespace: $namespace:ident,
        // optional feature for the variant
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

    (@macro_rules [ $({
        variant: $variant:ident,
        mod_name: $mod_name:ident,
        $(feature: $feature:literal,)?
    }),* ]) => {

        paste::paste! {
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

    };
}

pub (in crate::adapter) use define_adapter_kinds;
