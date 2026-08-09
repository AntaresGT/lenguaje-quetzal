//! Prompt `qz>` del REPL.

use std::borrow::Cow;

use reedline::{Prompt, PromptEditMode, PromptHistorySearch};

/// Prompt fijo: `qz> ` y `...> ` en continuaciones multilínea.
pub struct PromptQuetzal;

impl Prompt for PromptQuetzal {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed("qz")
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _modo: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("> ")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("...> ")
    }

    fn render_prompt_history_search_indicator(
        &self,
        _busqueda: PromptHistorySearch,
    ) -> Cow<'_, str> {
        Cow::Borrowed("? ")
    }
}
