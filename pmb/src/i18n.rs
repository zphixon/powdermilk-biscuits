use kdl::KdlDocument;
use once_cell::sync::Lazy;
use std::sync::RwLock;

macro_rules! messages {
    ($($variant:ident),* $(,)?) => {
        #[derive(Clone, Copy, Debug)]
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
    AskToSaveBeforeClosing,
    SaveUnnamedFile,
    OutOfMemory,
    CouldNotOpenConfigFile,
    ClearColor,
    StrokeColor,
    Pen,
    Pan,
    Eraser,
    UseMouseForPen,
    ToolForGesture1,
    Zoom,
    BrushSize,
    Modified,
    TitleUnmodifiedNoFile,
    TitleModifiedNoFile,
    FileMenu,
    FileNew,
    FileOpen,
    FileSave,
    FileSaveUnnamed,
    FileSettings,
    EditMenu,
    EditUndo,
    EditRedo,
);

#[macro_export]
macro_rules! s {
    ($variant:ident) => {
        $crate::i18n::get_str($crate::i18n::Message::$variant).to_string()
    };
    (&$variant:ident) => {
        $crate::i18n::get_str($crate::i18n::Message::$variant)
    };
}

static POT: Lazy<KdlDocument> = Lazy::new(|| {
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/pot.kdl"))
        .parse()
        .expect("pot.kdl failed to parse")
});

static LANGS: Lazy<Vec<String>> = Lazy::new(|| whoami::lang().collect());
static LANG: Lazy<RwLock<String>> = Lazy::new(|| {
    let lang = LANGS.first().expect("user has no preferred language");
    RwLock::new(lang.to_string())
});

pub fn get_lang() -> String {
    LANG.read().unwrap().clone()
}

pub fn set_lang(requested: &str) {
    if let Some(lang) = POT
        .nodes()
        .iter()
        .map(|node| node.name().value())
        // TODO probably split on dashes and compare the first segment
        .find(|lang| requested.starts_with(lang) || lang.starts_with(requested))
    {
        log::info!("setting language to {} (matches {})", lang, requested,);
        *LANG.try_write().expect("multithreaded set_lang") = lang.to_string();
    } else {
        panic!("no matching language for {}", requested);
    }
}

pub fn get_str(key: Message) -> &'static str {
    let lang = LANG.read().unwrap();
    POT.get(&lang)
        .unwrap_or_else(|| panic!("missing language {}", lang))
        .children()
        .unwrap_or_else(|| panic!("language {} has no messages", lang))
        .get(key.as_str())
        .unwrap_or_else(|| panic!("language {} missing messages {:?}", lang, key))
        .entries()
        .get(0)
        .unwrap_or_else(|| panic!("language {} message {:?} missing value", lang, key))
        .value()
        .as_string()
        .unwrap_or_else(|| panic!("language {} message {:?} is not a string", lang, key))
}

#[cfg(test)]
mod test {
    use super::*;
    use kdl::KdlValue;
    use std::collections::HashSet;

    #[test]
    fn test() {
        let pot = &*POT;
        let required = Message::all_strs()
            .iter()
            .fold(HashSet::new(), |mut set, required| {
                set.insert(*required);
                set
            });

        pot.nodes().iter().for_each(|lang| {
            let names = lang
                .children()
                .unwrap()
                .nodes()
                .iter()
                .map(|message| {
                    assert_eq!(
                        1,
                        message.entries().len(),
                        "language {} message {} should have exactly one translation",
                        lang.name().value(),
                        message.name().value(),
                    );

                    assert!(
                        matches!(
                            message.entries()[0].value(),
                            KdlValue::String(_) | KdlValue::RawString(_)
                        ),
                        "language {} message {} must be a string",
                        lang.name().value(),
                        message.name().value(),
                    );

                    message.name().value()
                })
                .collect::<HashSet<_>>();

            let d1 = required.difference(&names).collect::<Vec<_>>();
            let d2 = names.difference(&required).collect::<Vec<_>>();

            assert!(
                d1.is_empty(),
                "language {} missing translations {:?}",
                lang.name().value(),
                d1
            );

            assert!(
                d2.is_empty(),
                "language {} has extraneous translations {:?}",
                lang.name().value(),
                d2
            );
        });
    }
}
