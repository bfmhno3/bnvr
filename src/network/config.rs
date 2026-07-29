use std::error::Error;

use serde_yaml::{Mapping, Value};

pub fn inject_tun_config(
    base_yaml: &str,
    device_name: &str,
    bypass_routes: &[String],
) -> Result<String, Box<dyn Error>> {
    let mut value: Value = serde_yaml::from_str(base_yaml)?;
    let mapping = value
        .as_mapping_mut()
        .ok_or("invalid config: expected a YAML mapping")?;

    mapping.insert(key("tun"), tun_config(device_name));
    mapping.insert(key("dns"), dns_config());
    inject_bypass_rules(mapping, bypass_routes);

    serde_yaml::to_string(&value).map_err(Into::into)
}

fn inject_bypass_rules(mapping: &mut Mapping, bypass_routes: &[String]) {
    if bypass_routes.is_empty() {
        return;
    }

    let existing = mapping.remove(key("rules"));
    let mut rules = bypass_routes
        .iter()
        .map(|target| Value::String(format!("IP-CIDR,{target},DIRECT")))
        .collect::<Vec<_>>();

    if let Some(Value::Sequence(existing_rules)) = existing {
        rules.extend(existing_rules);
    }

    mapping.insert(key("rules"), Value::Sequence(rules));
}

fn tun_config(device_name: &str) -> Value {
    let mut tun = Mapping::new();
    tun.insert(key("enable"), Value::Bool(true));
    tun.insert(key("device"), Value::String(device_name.to_string()));
    tun.insert(key("stack"), Value::String("system".to_string()));
    tun.insert(key("auto-route"), Value::Bool(false));
    tun.insert(key("auto-detect-interface"), Value::Bool(false));
    tun.insert(key("dns-hijack"), Value::Sequence(Vec::new()));
    Value::Mapping(tun)
}

fn dns_config() -> Value {
    let mut dns = Mapping::new();
    dns.insert(key("enable"), Value::Bool(true));
    dns.insert(key("listen"), Value::String("198.18.0.1:53".to_string()));
    dns.insert(key("enhanced-mode"), Value::String("fake-ip".to_string()));
    dns.insert(
        key("fake-ip-range"),
        Value::String("198.18.0.0/15".to_string()),
    );
    dns.insert(
        key("nameserver"),
        string_sequence(&["223.5.5.5", "119.29.29.29"]),
    );
    dns.insert(
        key("fallback"),
        string_sequence(&["tls://1.1.1.1:853", "tls://8.8.8.8:853"]),
    );

    let mut fallback_filter = Mapping::new();
    fallback_filter.insert(key("geoip"), Value::Bool(true));
    fallback_filter.insert(key("geoip-code"), Value::String("CN".to_string()));
    fallback_filter.insert(key("ipcidr"), string_sequence(&["240.0.0.0/4"]));
    dns.insert(key("fallback-filter"), Value::Mapping(fallback_filter));

    Value::Mapping(dns)
}

fn string_sequence(items: &[&str]) -> Value {
    Value::Sequence(
        items
            .iter()
            .map(|item| Value::String((*item).to_string()))
            .collect(),
    )
}

fn key(key: &str) -> Value {
    Value::String(key.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_tun_config_adds_tun_dns_and_bypass_rules() {
        let routes = vec!["192.168.1.0/24".to_string(), "10.0.0.1/32".to_string()];
        let result = inject_tun_config(
            "proxies: []\nrules:\n  - MATCH,Proxy\n",
            "tun-bnvr0",
            &routes,
        )
        .unwrap();
        let value: Value = serde_yaml::from_str(&result).unwrap();
        let map = value.as_mapping().unwrap();

        assert_eq!(map.get(key("tun")).unwrap()["device"], "tun-bnvr0");
        assert_eq!(map.get(key("dns")).unwrap()["enhanced-mode"], "fake-ip");
        let rules = map.get(key("rules")).unwrap().as_sequence().unwrap();
        assert_eq!(rules[0], "IP-CIDR,192.168.1.0/24,DIRECT");
        assert_eq!(rules[1], "IP-CIDR,10.0.0.1/32,DIRECT");
        assert_eq!(rules[2], "MATCH,Proxy");
    }
}
