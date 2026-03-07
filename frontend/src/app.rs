use yew::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub container_id: Option<String>,
}

#[function_component(Header)]
pub fn header() -> Html {
    html! {
        <header class="header">
            <div class="container">
                <h1>{"📦 Loggy - Docker Observability (WASM)"}</h1>
            </div>
        </header>
    }
}

#[function_component(ContainersPanel)]
pub fn containers_panel() -> Html {
    html! {
        <div class="card">
            <h2>{"Containers"}</h2>
            <div class="loading">{"Connect to API to view containers"}</div>
        </div>
    }
}

#[function_component(LogViewer)]
pub fn log_viewer() -> Html {
    html! {
        <div class="card">
            <h2>{"Real-time Logs"}</h2>
            <span class="ws-status">{"Waiting for WebSocket..."}</span>
            <div class="log-container">
                <div class="loading">{"Logs will appear here when connected"}</div>
            </div>
        </div>
    }
}

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <div class="app">
            <Header />
            <main class="container">
                <div class="grid">
                    <ContainersPanel />
                    <LogViewer />
                </div>
            </main>
        </div>
    }
}
