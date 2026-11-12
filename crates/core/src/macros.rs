//! Macros that remove repeated code from the domain's textual types.
//!
//! - [`text_enum!`](crate::text_enum): an enum whose canonical text (JSON, SQLite, prompts)
//!   is declared ONCE. Exported: use it in any crate for enums with a canonical text form.
//! - `impl_text!` (internal): `Display`, `AsRef<str>` and serde for types that already
//!   have `as_str()` and `FromStr` (the identifiers in [`crate::ids`]).

/// Declares an enum whose canonical text representation is defined exactly once.
///
/// Generates: `ALL`, `as_str()`, `Display`, `FromStr` (error [`crate::Error::UnknownValue`]),
/// `Serialize` and `Deserialize` (as a string). Declaration order defines `Ord`.
///
/// ```
/// opentatu_core::text_enum! {
///     /// Example.
///     pub enum Color {
///         /// Blue.
///         Blue => "BLUE",
///         /// Green.
///         Green => "GREEN",
///     }
/// }
/// assert_eq!(Color::Blue.as_str(), "BLUE");
/// assert_eq!("GREEN".parse::<Color>().unwrap(), Color::Green);
/// ```
#[macro_export]
macro_rules! text_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident => $text:literal ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum $name {
            $( $(#[$vmeta])* $variant ),+
        }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &'static [$name] = &[ $( $name::$variant ),+ ];

            /// Canonical text (the same one used in JSON, SQLite and prompts).
            pub const fn as_str(self) -> &'static str {
                match self {
                    $( $name::$variant => $text ),+
                }
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = $crate::Error;

            fn from_str(text: &str) -> ::std::result::Result<Self, Self::Err> {
                match text {
                    $( $text => Ok($name::$variant), )+
                    _ => Err($crate::Error::UnknownValue {
                        kind: stringify!($name),
                        value: text.to_owned(),
                    }),
                }
            }
        }

        impl $crate::__serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
            where
                S: $crate::__serde::Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> $crate::__serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
            where
                D: $crate::__serde::Deserializer<'de>,
            {
                let text =
                    <::std::string::String as $crate::__serde::Deserialize>::deserialize(deserializer)?;
                text.parse().map_err(<D::Error as $crate::__serde::de::Error>::custom)
            }
        }
    };
}

/// Implements `Display`, `AsRef<str>`, `Serialize` and `Deserialize` for a textual type
/// that already has `fn as_str(&self) -> &str` and `FromStr<Err = Error>`.
macro_rules! impl_text {
    ($type:ty) => {
        impl ::std::fmt::Display for $type {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl AsRef<str> for $type {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl ::serde::Serialize for $type {
            fn serialize<S: ::serde::Serializer>(
                &self,
                serializer: S,
            ) -> ::std::result::Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $type {
            fn deserialize<D: ::serde::Deserializer<'de>>(
                deserializer: D,
            ) -> ::std::result::Result<Self, D::Error> {
                let text = <String as ::serde::Deserialize>::deserialize(deserializer)?;
                text.parse().map_err(::serde::de::Error::custom)
            }
        }
    };
}

pub(crate) use impl_text;
