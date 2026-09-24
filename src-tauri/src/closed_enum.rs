/// One list gives the enum, its `ALL` array, its stored spelling, `Display` and
/// `FromStr`. The literal per variant is the single spelling: serde, the column,
/// `Display` and `FromStr` all read it, so they cannot drift.
macro_rules! closed_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident = $text:literal
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(
            Debug,
            Clone,
            Copy,
            Default,
            PartialEq,
            Eq,
            Hash,
            serde::Serialize,
            serde::Deserialize,
            specta::Type,
        )]
        $vis enum $name {
            $(
                $(#[$variant_meta])*
                #[serde(rename = $text)]
                $variant,
            )+
        }

        impl $name {
            /// Every variant, in declaration order — what lets a test assert over the
            /// whole set, so adding one cannot escape the round-trip checks.
            #[allow(dead_code)]
            $vis const ALL: [Self; [$(Self::$variant),+].len()] = [$(Self::$variant),+];

            $vis fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        /// The column carries no `CHECK`: a database written by a newer version may hold
        /// a value this one never heard of, and the caller decides to fall back.
        impl std::str::FromStr for $name {
            type Err = ();

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($text => Ok(Self::$variant),)+
                    _ => Err(()),
                }
            }
        }
    };
}

pub(crate) use closed_enum;
