//! Kraken Spot REST AddOrder request types.
//!
//! Kraken's REST AddOrder is a form endpoint (POST `application/x-www-form-urlencoded`)
//! using v1 field names: `type` / `ordertype` / `price` / `price2` / `oflags` /
//! `starttm` / `expiretm` / `timeinforce` / `close[...]`. The struct keeps that
//! 1:1 shape so unfamiliar fields trace cleanly back to Kraken docs.
//!
//! `validate()` runs every Kraken-side precondition before issuing the call so
//! a malformed order rejects locally instead of generating an opaque API error.

use super::helpers::{
    format_f64_param, is_supported_kraken_close_order_type, is_supported_kraken_order_type,
    normalize_kraken_order_type, push_opt_param, requires_primary_price, requires_secondary_price,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct KrakenConditionalClose {
    pub order_type: String,
    pub price: Option<String>,
    pub price2: Option<String>,
}

impl KrakenConditionalClose {
    pub fn new(order_type: impl Into<String>) -> Self {
        Self {
            order_type: order_type.into(),
            price: None,
            price2: None,
        }
    }
}

/// Full Kraken Spot REST AddOrder request.
///
/// This uses Kraken's REST/WebSocket v1 field names because REST AddOrder is a
/// form endpoint: `type`, `ordertype`, `price`, `price2`, `oflags`, `starttm`,
/// `expiretm`, `timeinforce`, and `close[...]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct KrakenOrderRequest {
    pub pair: String,
    pub side: String,
    pub order_type: String,
    pub volume: String,
    pub price: Option<String>,
    pub price2: Option<String>,
    pub display_volume: Option<String>,
    pub leverage: Option<String>,
    pub margin: Option<bool>,
    pub reduce_only: bool,
    pub oflags: Vec<String>,
    pub start_time: Option<String>,
    pub expire_time: Option<String>,
    pub deadline: Option<String>,
    pub client_order_id: Option<String>,
    pub userref: Option<String>,
    pub sender_sub_id: Option<String>,
    pub stp_type: Option<String>,
    pub validate: bool,
    pub time_in_force: Option<String>,
    pub close: Option<KrakenConditionalClose>,
    pub req_id: Option<i64>,
}

impl KrakenOrderRequest {
    pub fn basic(
        pair: impl Into<String>,
        side: impl Into<String>,
        order_type: impl Into<String>,
        volume: f64,
    ) -> Self {
        Self {
            pair: pair.into(),
            side: side.into(),
            order_type: order_type.into(),
            volume: format_f64_param(volume),
            ..Self::default()
        }
    }

    pub fn with_price(mut self, price: f64) -> Self {
        self.price = Some(format_f64_param(price));
        self
    }

    pub fn with_price2(mut self, price2: f64) -> Self {
        self.price2 = Some(format_f64_param(price2));
        self
    }

    pub fn with_display_volume(mut self, display_volume: f64) -> Self {
        self.display_volume = Some(format_f64_param(display_volume));
        self
    }

    fn normalized_order_type(&self) -> String {
        normalize_kraken_order_type(&self.order_type)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.pair.trim().is_empty() {
            return Err("Kraken order pair is required".to_string());
        }
        if !matches!(self.side.to_ascii_lowercase().as_str(), "buy" | "sell") {
            return Err(format!("Unsupported Kraken order side: {}", self.side));
        }
        let order_type = self.normalized_order_type();
        if !is_supported_kraken_order_type(&order_type) {
            return Err(format!(
                "Unsupported Kraken order type: {}",
                self.order_type
            ));
        }
        let volume = self
            .volume
            .parse::<f64>()
            .map_err(|_| format!("Invalid Kraken order volume: {}", self.volume))?;
        if !volume.is_finite() || (volume <= 0.0 && order_type != "settle-position") {
            return Err(format!("Invalid Kraken order volume: {}", self.volume));
        }
        if order_type == "iceberg" && self.display_volume.as_deref().unwrap_or("").is_empty() {
            return Err("Kraken iceberg order requires displayvol".to_string());
        }
        if requires_primary_price(&order_type) && self.price.as_deref().unwrap_or("").is_empty() {
            return Err(format!("Kraken {order_type} order requires price"));
        }
        if requires_secondary_price(&order_type) && self.price2.as_deref().unwrap_or("").is_empty()
        {
            return Err(format!("Kraken {order_type} order requires price2"));
        }
        if self.client_order_id.is_some() && self.userref.is_some() {
            return Err("Kraken cl_ord_id and userref are mutually exclusive".to_string());
        }
        if let Some(tif) = &self.time_in_force {
            let tif = tif.to_ascii_uppercase();
            if !matches!(tif.as_str(), "GTC" | "GTD" | "IOC") {
                return Err(format!("Unsupported Kraken timeinforce: {tif}"));
            }
        }
        if let Some(stp) = &self.stp_type {
            if !matches!(
                stp.to_ascii_lowercase().as_str(),
                "cancel_newest" | "cancel_oldest" | "cancel_both"
            ) {
                return Err(format!("Unsupported Kraken stp_type: {stp}"));
            }
        }
        if let Some(close) = &self.close {
            let close_type = normalize_kraken_order_type(&close.order_type);
            if !is_supported_kraken_close_order_type(&close_type) {
                return Err(format!(
                    "Unsupported Kraken conditional close order type: {}",
                    close.order_type
                ));
            }
            if requires_primary_price(&close_type)
                && close.price.as_deref().unwrap_or("").is_empty()
            {
                return Err(format!("Kraken conditional {close_type} requires price"));
            }
            if requires_secondary_price(&close_type)
                && close.price2.as_deref().unwrap_or("").is_empty()
            {
                return Err(format!("Kraken conditional {close_type} requires price2"));
            }
        }
        Ok(())
    }

    pub(super) fn to_params(&self) -> Vec<(String, String)> {
        let mut params = vec![
            ("pair".to_string(), self.pair.clone()),
            ("type".to_string(), self.side.to_ascii_lowercase()),
            ("ordertype".to_string(), self.rest_order_type()),
            ("volume".to_string(), self.volume.clone()),
        ];
        push_opt_param(&mut params, "price", self.price.as_deref());
        push_opt_param(&mut params, "price2", self.price2.as_deref());
        push_opt_param(&mut params, "displayvol", self.display_volume.as_deref());
        push_opt_param(&mut params, "leverage", self.leverage.as_deref());
        if let Some(margin) = self.margin {
            params.push(("margin".to_string(), margin.to_string()));
        }
        if self.reduce_only {
            params.push(("reduce_only".to_string(), "true".to_string()));
        }
        if !self.oflags.is_empty() {
            params.push(("oflags".to_string(), self.oflags.join(",")));
        }
        push_opt_param(&mut params, "starttm", self.start_time.as_deref());
        push_opt_param(&mut params, "expiretm", self.expire_time.as_deref());
        push_opt_param(&mut params, "deadline", self.deadline.as_deref());
        push_opt_param(&mut params, "cl_ord_id", self.client_order_id.as_deref());
        push_opt_param(&mut params, "userref", self.userref.as_deref());
        push_opt_param(&mut params, "sender_sub_id", self.sender_sub_id.as_deref());
        push_opt_param(&mut params, "stp_type", self.stp_type.as_deref());
        if self.validate {
            params.push(("validate".to_string(), "true".to_string()));
        }
        if let Some(tif) = &self.time_in_force {
            params.push(("timeinforce".to_string(), tif.to_ascii_uppercase()));
        }
        if let Some(close) = &self.close {
            params.push((
                "close[ordertype]".to_string(),
                normalize_kraken_order_type(&close.order_type),
            ));
            push_opt_param(&mut params, "close[price]", close.price.as_deref());
            push_opt_param(&mut params, "close[price2]", close.price2.as_deref());
        }
        if let Some(req_id) = self.req_id {
            params.push(("reqid".to_string(), req_id.to_string()));
        }
        params
    }

    fn rest_order_type(&self) -> String {
        let order_type = self.normalized_order_type();
        if order_type == "iceberg" {
            "limit".to_string()
        } else {
            order_type
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_constructor_normalizes_volume_format() {
        let req = KrakenOrderRequest::basic("XBTUSD", "buy", "market", 1.0);
        assert_eq!(req.volume, "1");
        let req = KrakenOrderRequest::basic("XBTUSD", "buy", "market", 1.5);
        assert_eq!(req.volume, "1.5");
    }

    #[test]
    fn validate_rejects_blank_pair() {
        let mut req = KrakenOrderRequest::basic("", "buy", "market", 1.0);
        req.pair = "".to_string();
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_rejects_unknown_side() {
        let req = KrakenOrderRequest::basic("XBTUSD", "long", "market", 1.0);
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_rejects_unknown_order_type() {
        let req = KrakenOrderRequest::basic("XBTUSD", "buy", "bracket", 1.0);
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_rejects_iceberg_without_displayvol() {
        let req = KrakenOrderRequest::basic("XBTUSD", "buy", "iceberg", 1.0).with_price(50_000.0);
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_accepts_iceberg_with_displayvol() {
        let req = KrakenOrderRequest::basic("XBTUSD", "buy", "iceberg", 10.0)
            .with_price(50_000.0)
            .with_display_volume(1.0);
        assert!(req.validate().is_ok());
    }

    #[test]
    fn validate_rejects_limit_without_price() {
        let req = KrakenOrderRequest::basic("XBTUSD", "buy", "limit", 1.0);
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_rejects_stop_loss_limit_without_price2() {
        let req =
            KrakenOrderRequest::basic("XBTUSD", "buy", "stop-loss-limit", 1.0).with_price(50_000.0);
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_rejects_market_with_zero_volume() {
        let req = KrakenOrderRequest::basic("XBTUSD", "buy", "market", 0.0);
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_rejects_clord_and_userref_together() {
        let mut req = KrakenOrderRequest::basic("XBTUSD", "buy", "market", 1.0);
        req.client_order_id = Some("abc".into());
        req.userref = Some("123".into());
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_rejects_unknown_timeinforce() {
        let mut req = KrakenOrderRequest::basic("XBTUSD", "buy", "limit", 1.0).with_price(50_000.0);
        req.time_in_force = Some("FOK".into());
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_accepts_gtc_ioc_gtd_timeinforce() {
        for tif in ["GTC", "GTD", "IOC", "gtc", "ioc"] {
            let mut req =
                KrakenOrderRequest::basic("XBTUSD", "buy", "limit", 1.0).with_price(50_000.0);
            req.time_in_force = Some(tif.into());
            assert!(req.validate().is_ok(), "{tif} should be accepted");
        }
    }

    #[test]
    fn to_params_emits_kraken_field_names() {
        let req = KrakenOrderRequest::basic("XBTUSD", "buy", "limit", 1.0).with_price(50_000.0);
        let params = req.to_params();
        let map: std::collections::HashMap<_, _> = params.into_iter().collect();
        assert_eq!(map.get("pair").map(String::as_str), Some("XBTUSD"));
        assert_eq!(map.get("type").map(String::as_str), Some("buy"));
        assert_eq!(map.get("ordertype").map(String::as_str), Some("limit"));
        assert_eq!(map.get("volume").map(String::as_str), Some("1"));
        assert_eq!(map.get("price").map(String::as_str), Some("50000"));
    }

    #[test]
    fn to_params_maps_iceberg_to_limit_with_displayvol() {
        let req = KrakenOrderRequest::basic("XBTUSD", "buy", "iceberg", 10.0)
            .with_price(50_000.0)
            .with_display_volume(1.0);
        let map: std::collections::HashMap<_, _> = req.to_params().into_iter().collect();
        assert_eq!(map.get("ordertype").map(String::as_str), Some("limit"));
        assert_eq!(map.get("displayvol").map(String::as_str), Some("1"));
    }

    #[test]
    fn to_params_emits_close_subfields() {
        let mut req = KrakenOrderRequest::basic("XBTUSD", "buy", "limit", 1.0).with_price(50_000.0);
        req.close = Some(KrakenConditionalClose {
            order_type: "stop-loss".into(),
            price: Some("48000".into()),
            price2: None,
        });
        let map: std::collections::HashMap<_, _> = req.to_params().into_iter().collect();
        assert_eq!(
            map.get("close[ordertype]").map(String::as_str),
            Some("stop-loss")
        );
        assert_eq!(map.get("close[price]").map(String::as_str), Some("48000"));
    }
}
