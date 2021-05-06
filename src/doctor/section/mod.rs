pub mod android;
#[cfg(target_os = "macos")]
pub mod apple;
pub mod cargo_mobile;
pub mod device_list;

use crate::util::{
    self,
    cli::{colors, TextWrapper},
};
use colored::Colorize as _;
use std::fmt::Debug;
use thiserror::Error;

fn command(command: &str) -> Result<String, Error> {
    bossy::Command::impure_parse(command)
        .run_and_wait_for_str(|s| s.trim_end().to_owned())
        .map_err(Error::from)
}

#[derive(Debug, Error)]
enum Error {
    #[error("Failed to check installed macOS version")]
    OsCheckFailed(#[from] bossy::Error),
    #[error("Output contained invalid UTF-8: {0}")]
    InvalidUtf8(#[from] std::str::Utf8Error),
    #[error("Environment variable not set.")]
    VarError(#[from] std::env::VarError),
    #[error(transparent)]
    CommandSearchFailed(#[from] util::RunAndSearchError),
    #[error("iOS linking is broken on Rust versions later than 1.45.2 (d3fb005a3 2020-07-31) and earlier than 1.49.0-nightly (ffa2e7ae8 2020-10-24), but you're on {version}!\n    - Until this is resolved by Rust 1.49.0, please do one of the following:\n        A) downgrade to 1.45.2:\n           `rustup install stable-2020-08-03 && rustup default stable-2020-08-03`\n        B) update to a recent nightly:\n           `rustup update nightly && rustup default nightly`")]
    RustVersionInvalid { version: util::RustVersion },
    #[error("Commit message error")]
    InstalledCommitMsgFailed(#[from] util::InstalledCommitMsgError),
}

#[derive(Clone, Copy, Debug)]
enum Label {
    Victory,
    Warning,
    Error,
}

impl Label {
    fn title_symbol(self) -> &'static str {
        match self {
            Self::Victory | Self::Warning => "✔",
            Self::Error => "!",
        }
    }

    fn item_symbol(self) -> &'static str {
        match self {
            Self::Victory => "•",
            Self::Warning | Self::Error => "✗",
        }
    }

    fn color(self) -> colored::Color {
        match self {
            Self::Victory => colors::VICTORY,
            Self::Warning => colors::WARNING,
            Self::Error => colors::ERROR,
        }
    }

    fn format_title(self, title: &str) -> colored::ColoredString {
        format!("[{}] {}", self.title_symbol(), title)
            .color(self.color())
            .bold()
    }

    fn format_item(self, msg: &str) -> colored::ColoredString {
        let item = format!("{} {}", self.item_symbol(), msg);
        match self {
            Self::Victory => item.normal(),
            _ => item.color(self.color()).bold(),
        }
    }
}

#[derive(Debug)]
struct Item {
    label: Label,
    msg: String,
}

impl Item {
    fn new(label: Label, msg: impl ToString) -> Self {
        Self {
            label,
            msg: msg.to_string(),
        }
    }

    fn victory(msg: impl ToString) -> Self {
        Self::new(Label::Victory, msg)
    }

    fn warning(msg: impl ToString) -> Self {
        Self::new(Label::Warning, msg)
    }

    fn failure(msg: impl ToString) -> Self {
        Self::new(Label::Error, msg)
    }

    fn from_result(result: Result<impl ToString, impl Into<Error>>) -> Self {
        util::unwrap_either(
            result
                .map(Self::victory)
                .map_err(|err| Self::failure(err.into())),
        )
    }

    fn is_warning(&self) -> bool {
        matches!(self.label, Label::Warning)
    }

    fn is_failure(&self) -> bool {
        matches!(self.label, Label::Error)
    }

    fn format(&self) -> colored::ColoredString {
        self.label.format_item(&self.msg)
    }
}

#[derive(Debug)]
pub struct Section {
    title: String,
    items: Vec<Item>,
}

impl Section {
    fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Default::default(),
        }
    }

    fn add_item(&mut self, item: Item) -> &mut Self {
        self.items.push(item);
        self
    }

    fn with_item(mut self, item: Item) -> Self {
        self.add_item(item);
        self
    }

    fn add_items(&mut self, items: impl IntoIterator<Item = Item>) -> &mut Self {
        self.items.extend(items);
        self
    }

    fn with_items(mut self, items: impl IntoIterator<Item = Item>) -> Self {
        self.add_items(items);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn has_error(&self) -> bool {
        self.items.iter().any(Item::is_failure)
    }

    fn has_warning(&self) -> bool {
        self.items.iter().any(Item::is_warning)
    }

    fn label(&self) -> Label {
        if self.has_error() {
            Label::Error
        } else if self.has_warning() {
            Label::Warning
        } else {
            Label::Victory
        }
    }

    pub fn print(&self, wrapper: &TextWrapper) {
        static BULLET_INDENT: &str = "    ";
        static HANGING_INDENT: &str = "      ";
        let bullet_wrapper = wrapper
            .clone()
            .initial_indent(BULLET_INDENT)
            .subsequent_indent(HANGING_INDENT);
        println!(
            "{}",
            // The `.to_string()` at the end is necessary for the color/bold to
            // actually show - otherwise, the colored string just `AsRef`s to
            // satisfy `TextWrapper::fill` and the formatting is left behind.
            wrapper.fill(&self.label().format_title(&self.title).to_string())
        );
        for report_bullet in &self.items {
            println!(
                "{}",
                bullet_wrapper.fill(&report_bullet.format().to_string())
            );
        }
    }
}
