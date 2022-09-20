use kdl::KdlDocument;
use once_cell::sync::Lazy;

static POT: Lazy<KdlDocument> = Lazy::new(|| {
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/pot.kdl"))
        .parse()
        .expect("pot.kdl failed to parse")
});

static LANG: Lazy<String> = Lazy::new(|| {
    #[cfg(unix)]
    {
        std::env::var("LANGUAGE")
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_else(|_| String::from("en_US.UTF-8"))
    }

    #[cfg(windows)]
    {
        todo!()
    }

    // TODO wasm??
});

macro_rules! messages {
    ($($variant:ident),* $(,)?) => {
        #[derive(Clone, Copy)]
        pub enum Message {
            $($variant),*
        }

        impl Message {
            #[cfg(test)]
            fn all_strs() -> &'static [&'static str] {
                &[$(stringify!($variant)),*]
            }

            fn as_str(&self) -> &'static str {
                use Message::*;
                match self {
                    $($variant => stringify!($variant)),*
                }
            }
        }
    };
}

messages!(
    MigrateWarningTitle,
    MigrateWarningMessage,
    ErrorTitle,
    UnsavedChangesTitle,
    OpenTitle,
    RealHotItem,
    CouldNotOpenFile,
    CouldNotSaveFile,
    AskToSaveBeforeOpening,
    SaveUnnamedFile,
    OutOfMemory,
    CouldNotOpenConfigFile,
);

#[macro_export]
macro_rules! s {
    ($variant:ident) => {
        $crate::i18n::get_str($crate::i18n::Message::$variant)
    };
    (&$variant:ident) => {
        $crate::i18n::get_str($crate::i18n::Message::$variant).as_str()
    };
}

pub fn get_str(key: Message) -> String {
    let lang = &*LANG;
    let pot = &*POT;

    pot.get(&lang)
        .unwrap_or_else(|| panic!("missing language {}", lang))
        .get(key.as_str())
        .unwrap_or_else(|| panic!("missing language {}", lang))
        .to_string()
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test() {
        let pot = &*POT;
        let required = Message::all_strs();

        for lang in pot.nodes() {
            let children = lang
                .children()
                .unwrap()
                .nodes()
                .iter()
                .map(|node| {
                    assert!(
                        !node.is_empty(),
                        "language {} missing content for message {}",
                        lang.name().to_string(),
                        node.name().to_string()
                    );

                    node.name().to_string()
                })
                .collect::<HashSet<_>>();

            for required in required {
                assert!(
                    children.contains(*required),
                    "language {} missing message {}",
                    lang.name().to_string(),
                    required
                );
            }
        }
    }
}
