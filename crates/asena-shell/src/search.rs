use asena_core::{
    apps::{AppIndex, DesktopEntry},
    search::{app_score, calculate, rank},
    usage::Usage,
};
#[derive(Clone)]
pub enum Action {
    Application(DesktopEntry),
    /// Settings panel identifier from `asena_core::panels`.
    Settings(String),
    Copy(String),
    Command(String),
}
pub struct Query {
    pub text: String,
}
pub struct SearchResult {
    pub title: String,
    pub icon: Option<String>,
    pub score: u32,
    pub action: Action,
}
pub trait SearchProvider {
    async fn query(&self, query: &Query) -> Vec<SearchResult>;
}
pub struct Applications {
    pub index: AppIndex,
    pub usage: Usage,
}
impl SearchProvider for Applications {
    async fn query(&self, query: &Query) -> Vec<SearchResult> {
        if query.text.len() > 256 {
            return Vec::new();
        }
        let index = self.index.clone();
        let usage = self.usage.clone();
        let query = query.text.clone();
        gio::spawn_blocking(move || {
            index
                .entries()
                .iter()
                .filter(|app| app.visible)
                .filter_map(|app| {
                    let score = app_score(&query, app, usage.get(&app.id).copied().unwrap_or(0));
                    (score > 0).then(|| SearchResult {
                        title: app.name.clone(),
                        icon: app.icon.clone(),
                        score,
                        action: Action::Application(app.clone()),
                    })
                })
                .collect()
        })
        .await
        .unwrap_or_default()
    }
}
pub struct Calculator;
impl SearchProvider for Calculator {
    async fn query(&self, query: &Query) -> Vec<SearchResult> {
        calculate(&query.text)
            .map(|value| {
                let text = value.to_string();
                SearchResult {
                    title: text.clone(),
                    icon: Some("accessories-calculator-symbolic".into()),
                    score: 1000,
                    action: Action::Copy(text),
                }
            })
            .into_iter()
            .collect()
    }
}
pub struct Commands;
impl SearchProvider for Commands {
    async fn query(&self, query: &Query) -> Vec<SearchResult> {
        if query.text.len() > 4096 {
            return Vec::new();
        }
        query
            .text
            .strip_prefix('>')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|command| SearchResult {
                title: command.into(),
                icon: Some("utilities-terminal-symbolic".into()),
                score: 1000,
                action: Action::Command(command.into()),
            })
            .into_iter()
            .collect()
    }
}
pub struct Settings;
impl SearchProvider for Settings {
    async fn query(&self, query: &Query) -> Vec<SearchResult> {
        settings_results(&query.text)
    }
}
/// Settings panels whose name, identifier or keywords match every query word.
pub fn settings_results(text: &str) -> Vec<SearchResult> {
    let text = text.trim();
    if text.is_empty() || text.len() > 256 || text.starts_with('>') {
        return Vec::new();
    }
    asena_core::panels::all()
        .into_iter()
        .filter(|panel| panel.matches(text))
        .map(|panel| SearchResult {
            // Applications with a stronger title match stay above settings.
            score: rank(text, &panel.name).max(150).saturating_sub(10),
            title: panel.name,
            icon: Some(panel.icon.into()),
            action: Action::Settings(panel.id.into()),
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_search_routes_to_panels() {
        let results = settings_results("bluetooth");
        assert!(results
            .iter()
            .any(|result| matches!(&result.action, Action::Settings(id) if id == "bluetooth")));
        assert!(settings_results("").is_empty());
        assert!(settings_results("> ls").is_empty());
        assert!(settings_results("zzzz-not-a-setting").is_empty());
    }
}
