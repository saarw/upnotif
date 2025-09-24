use log::{error, info};
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tokio::time::{interval};
use url::Url;

#[derive(Debug, Clone, PartialEq)]
enum UrlStatus {
    Up,
    Down,
}

impl std::fmt::Display for UrlStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            UrlStatus::Up => write!(f, "UP"),
            UrlStatus::Down => write!(f, "DOWN"),
        }
    }
}

struct Config {
    urls: Vec<String>,
    slack_webhook: String,
    interval_seconds: u64,
    test_mode: bool,
}

impl Config {
    fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let urls_str = env::var("UPNOTIF_URLS")
            .map_err(|_| "UPNOTIF_URLS environment variable is required")?;

        let slack_webhook = env::var("UPNOTIF_SLACK_WEBHOOK")
            .map_err(|_| "UPNOTIF_SLACK_WEBHOOK environment variable is required")?;

        let interval_seconds = env::var("UPNOTIF_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "60".to_string())
            .parse::<u64>()
            .map_err(|_| "UPNOTIF_INTERVAL_SECONDS must be a valid number")?;

        let urls: Vec<String> = urls_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if urls.is_empty() {
            return Err("At least one URL must be provided in UPNOTIF_URLS".into());
        }

        // Validate URLs
        for url in &urls {
            Url::parse(url)
                .map_err(|_| format!("Invalid URL: {}", url))?;
        }

        let test_mode = slack_webhook == "test";

        // Validate Slack webhook URL (unless in test mode)
        if !test_mode {
            Url::parse(&slack_webhook)
                .map_err(|_| "Invalid Slack webhook URL")?;
        }

        Ok(Config {
            urls,
            slack_webhook,
            interval_seconds,
            test_mode,
        })
    }
}

struct UrlMonitor {
    client: Client,
    config: Config,
    status_map: HashMap<String, UrlStatus>,
}

impl UrlMonitor {
    fn new(config: Config) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            config,
            status_map: HashMap::new(),
        }
    }

    async fn check_url_status(&self, url: &str) -> UrlStatus {
        match self.client.get(url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    UrlStatus::Up
                } else {
                    UrlStatus::Down
                }
            }
            Err(_) => UrlStatus::Down,
        }
    }

    async fn send_notification(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.config.test_mode {
            // In test mode, just log to console
            info!("[TEST MODE] Slack notification: {}", message);
            Ok(())
        } else {
            // Send actual Slack notification
            let payload = json!({
                "text": message
            });

            let response = self
                .client
                .post(&self.config.slack_webhook)
                .json(&payload)
                .send()
                .await?;

            if !response.status().is_success() {
                return Err(format!("Slack webhook returned status: {}", response.status()).into());
            }

            Ok(())
        }
    }

    async fn check_all_urls(&mut self) -> Vec<(String, UrlStatus, bool)> {
        let mut results = Vec::new();

        for url in &self.config.urls {
            let current_status = self.check_url_status(url).await;
            let previous_status = self.status_map.get(url);
            let status_changed = previous_status.map_or(true, |prev| prev != &current_status);

            results.push((url.clone(), current_status.clone(), status_changed));
            self.status_map.insert(url.clone(), current_status);
        }

        results
    }

    async fn report_initial_status(&mut self) {
        info!("🚀 Starting URL monitoring...");

        let results = self.check_all_urls().await;
        let mut status_lines = Vec::new();

        for (url, status, _) in results {
            let emoji = match status {
                UrlStatus::Up => "✅",
                UrlStatus::Down => "❌",
            };
            let line = format!("{} {} is {}", emoji, url, status);
            info!("{}", line);
            status_lines.push(line);
        }

        let message = format!(
            "🔍 *URL Monitor Started*\nInitial status check:\n{}",
            status_lines.join("\n")
        );

        if let Err(e) = self.send_notification(&message).await {
            if self.config.test_mode {
                error!("Failed to log initial status: {}", e);
            } else {
                error!("Failed to send initial status to Slack: {}", e);
            }
        }
    }

    async fn monitor_urls(&mut self) {
        let mut interval_timer = interval(Duration::from_secs(self.config.interval_seconds));
        interval_timer.tick().await; // Skip the first tick

        loop {
            interval_timer.tick().await;

            let results = self.check_all_urls().await;
            let mut changes = Vec::new();

            for (url, status, status_changed) in results {
                if status_changed {
                    let emoji = match status {
                        UrlStatus::Up => "✅",
                        UrlStatus::Down => "❌",
                    };
                    let change_msg = format!("{} {} is now {}", emoji, url, status);
                    info!("Status change: {}", change_msg);
                    changes.push(change_msg);
                }
            }

            if !changes.is_empty() {
                let message = format!(
                    "🔔 *URL Status Changes*\n{}",
                    changes.join("\n")
                );

                if let Err(e) = self.send_notification(&message).await {
                    if self.config.test_mode {
                        error!("Failed to log status change: {}", e);
                    } else {
                        error!("Failed to send status change to Slack: {}", e);
                    }
                }
            }
        }
    }

    async fn run(&mut self) {
        self.report_initial_status().await;

        info!(
            "Monitoring {} URLs every {} seconds...",
            self.config.urls.len(),
            self.config.interval_seconds
        );

        self.monitor_urls().await;
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            error!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    info!("Configuration loaded successfully");
    info!("URLs to monitor: {:?}", config.urls);
    info!("Check interval: {} seconds", config.interval_seconds);
    if config.test_mode {
        info!("Running in TEST MODE - notifications will be logged to console instead of sent to Slack");
    }

    let mut monitor = UrlMonitor::new(config);
    monitor.run().await;
}