use crate::events::MatchingEvent;

#[derive(Clone, Debug)]
pub enum EventProducer {
    Kafka { hosts: String },
    Noop,
}

impl EventProducer {
    pub fn from_kafka_hosts(hosts: Option<String>) -> Self {
        match hosts {
            Some(h) if !h.is_empty() => {
                tracing::info!(hosts = %h, "Kafka event producer initialized");
                EventProducer::Kafka { hosts: h }
            }
            _ => {
                tracing::warn!("No Kafka configured, events will not be published externally");
                EventProducer::Noop
            }
        }
    }

    pub fn publish(&self, event: &MatchingEvent) {
        let payload = match serde_json::to_string(event) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize MatchingEvent");
                return;
            }
        };

        match self {
            EventProducer::Kafka { hosts } => {
                if let Err(e) = self.try_publish_kafka(hosts, &payload) {
                    tracing::warn!(error = %e, "Kafka publish failed");
                }
            }
            EventProducer::Noop => {
                tracing::debug!(event = %payload, "Event (noop)");
            }
        }
    }

    fn try_publish_kafka(&self, hosts: &str, payload: &str) -> Result<(), String> {
        tracing::debug!(hosts = %hosts, payload = %payload, "Kafka event (would publish)");
        tracing::info!(
            hosts = %hosts,
            payload_len = %payload.len(),
            "Kafka publishing attempted - add rdkafka dependency for production"
        );
        Err("Kafka not yet connected - install rdkafka crate for production use".to_string())
    }
}
