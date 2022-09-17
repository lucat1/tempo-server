use crate::{TAGGER_LOGLEVEL, TAGGER_STYLE};
use dialoguer::console::{style, Style, StyledObject};
use dialoguer::theme::Theme;
use env_logger::{fmt::Color, Builder, Env};
use log::{Level, LevelFilter};
use std::fmt;
use std::io::Write;

pub fn init_logger() {
    let env = Env::default()
        .default_filter_or(TAGGER_LOGLEVEL)
        .write_style(TAGGER_STYLE);

    Builder::from_env(env)
        .filter_level(LevelFilter::Info)
        .filter(Some("sqlx"), LevelFilter::Warn)
        .format(|buf, record| {
            let mut style = buf.style();
            let level = match record.level() {
                Level::Warn => style.set_color(Color::Yellow).value("   warn"),
                Level::Info => style.set_color(Color::Green).value("   info"),
                Level::Error => style.set_color(Color::Red).value("   error"),
                Level::Debug => style.set_color(Color::Blue).value("   debug"),
                Level::Trace => style
                    .set_color(Color::Blue)
                    .set_bold(true)
                    .value("   trace"),
            };

            writeln!(buf, "{} {}", level, record.args())
        })
        .init();
}

pub struct DialoguerTheme {
    /// The style for default values
    pub defaults_style: Style,
    /// The style for prompt
    pub prompt_style: Style,
    /// The style for prompt after the answer has been given
    pub prompt_selection_style: Style,
    /// Prompt prefix value and style
    pub prompt_prefix: StyledObject<String>,
    /// Prompt on success prefix value and style
    pub success_prefix: StyledObject<String>,
    /// The style for hints
    pub hint_style: Style,
    /// The style for values on prompt success
    pub values_style: Style,
    /// The style for active items
    pub active_item_style: Style,
    /// The style for inactive items
    pub inactive_item_style: Style,
    /// Active item in select prefix value and style
    pub active_item_prefix: StyledObject<String>,
    /// Inctive item in select prefix value and style
    pub inactive_item_prefix: StyledObject<String>,
    /// Checked item in multi select prefix value and style
    pub checked_item_prefix: StyledObject<String>,
    /// Unchecked item in multi select prefix value and style
    pub unchecked_item_prefix: StyledObject<String>,
    /// Show the selections from certain prompts inline
    pub inline_selections: bool,
}

impl Default for DialoguerTheme {
    fn default() -> Self {
        Self {
            defaults_style: Style::new().for_stderr().cyan(),
            prompt_style: Style::new().for_stderr().bold(),
            prompt_selection_style: Style::new().for_stderr(),
            prompt_prefix: style("      ?".to_string()).for_stderr().yellow(),
            success_prefix: style(" answer".to_string()).for_stderr().green(),
            hint_style: Style::new().for_stderr().black().bright(),
            values_style: Style::new().for_stderr().green(),
            active_item_style: Style::new().for_stderr().cyan(),
            inactive_item_style: Style::new().for_stderr(),
            active_item_prefix: style("      >".to_string()).for_stderr().green(),
            inactive_item_prefix: style("       ".to_string()).for_stderr(),
            checked_item_prefix: style("      o".to_string()).for_stderr().green(),
            unchecked_item_prefix: style("      o".to_string()).for_stderr().black(),
            inline_selections: true,
        }
    }
}

impl Theme for DialoguerTheme {
    /// Formats a prompt.
    fn format_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        write!(
            f,
            "{} {} ",
            &self.prompt_prefix,
            self.prompt_style.apply_to(prompt)
        )
    }

    /// Formats an input prompt.
    fn format_input_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        default: Option<&str>,
    ) -> fmt::Result {
        if !prompt.is_empty() {
            write!(
                f,
                "{} {} ",
                &self.prompt_prefix,
                self.prompt_style.apply_to(prompt)
            )?;
        }

        match default {
            Some(default) => write!(
                f,
                "{} ",
                self.hint_style.apply_to(&format!("({})", default)),
            ),
            None => write!(f, ""),
        }
    }

    /// Formats a confirm prompt.
    fn format_confirm_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        default: Option<bool>,
    ) -> fmt::Result {
        if !prompt.is_empty() {
            write!(
                f,
                "{} {} ",
                &self.prompt_prefix,
                self.prompt_style.apply_to(prompt)
            )?;
        }

        match default {
            None => write!(f, "{}", self.hint_style.apply_to("(y/n)"),),
            Some(true) => write!(
                f,
                "{} {}",
                self.hint_style.apply_to("(y/n)"),
                self.defaults_style.apply_to("yes")
            ),
            Some(false) => write!(
                f,
                "{} {}",
                self.hint_style.apply_to("(y/n)"),
                self.defaults_style.apply_to("no")
            ),
        }
    }

    /// Formats a confirm prompt after selection.
    fn format_confirm_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selection: Option<bool>,
    ) -> fmt::Result {
        if !prompt.is_empty() {
            write!(
                f,
                "{} {} ",
                &self.success_prefix,
                self.prompt_selection_style.apply_to(prompt)
            )?;
        }
        let selection = selection.map(|b| if b { "yes" } else { "no" });

        match selection {
            Some(selection) => {
                write!(f, "{}", self.values_style.apply_to(selection))
            }
            None => Ok(()),
        }
    }

    /// Formats an input prompt after selection.
    fn format_input_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        sel: &str,
    ) -> fmt::Result {
        if !prompt.is_empty() {
            write!(
                f,
                "{} {} ",
                &self.success_prefix,
                self.prompt_selection_style.apply_to(prompt)
            )?;
        }

        write!(f, "{}", self.values_style.apply_to(sel))
    }

    /// Formats a multi select prompt after selection.
    fn format_multi_select_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selections: &[&str],
    ) -> fmt::Result {
        if !prompt.is_empty() {
            write!(
                f,
                "{} {} ",
                &self.success_prefix,
                self.prompt_selection_style.apply_to(prompt)
            )?;
        }

        if self.inline_selections {
            for (idx, sel) in selections.iter().enumerate() {
                write!(
                    f,
                    "{}{}",
                    if idx == 0 { "" } else { ", " },
                    self.values_style.apply_to(sel)
                )?;
            }
        }

        Ok(())
    }

    /// Formats a select prompt item.
    fn format_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        active: bool,
    ) -> fmt::Result {
        let details = if active {
            (
                &self.active_item_prefix,
                self.active_item_style.apply_to(text),
            )
        } else {
            (
                &self.inactive_item_prefix,
                self.inactive_item_style.apply_to(text),
            )
        };

        write!(f, "{} {}", details.0, details.1)
    }

    /// Formats a multi select prompt item.
    fn format_multi_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        checked: bool,
        active: bool,
    ) -> fmt::Result {
        let details = match (checked, active) {
            (true, true) => (
                &self.checked_item_prefix,
                self.active_item_style.apply_to(text),
            ),
            (true, false) => (
                &self.checked_item_prefix,
                self.inactive_item_style.apply_to(text),
            ),
            (false, true) => (
                &self.unchecked_item_prefix,
                self.active_item_style.apply_to(text),
            ),
            (false, false) => (
                &self.unchecked_item_prefix,
                self.inactive_item_style.apply_to(text),
            ),
        };

        write!(f, "{} {}", details.0, details.1)
    }
}
