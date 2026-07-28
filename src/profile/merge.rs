use std::collections::{HashMap, HashSet};
use std::error::Error;

use serde_yaml::{Mapping, Value};
use tracing::{info, warn};

use super::crud::{self, ProfileKind, ProfileMeta};
use crate::paths;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct MergeStats {
    pub proxies: usize,
    pub dropped: usize,
    pub groups: usize,
    pub rules: usize,
}

#[derive(Debug)]
pub struct MergeResult {
    pub name: String,
    pub stats: MergeStats,
}

type ProxyKey = (String, String, String, String);

pub fn merge_documents(docs: &[(String, Value)]) -> Result<(Value, MergeStats), Box<dyn Error>> {
    let mut out = Mapping::new();
    let mut out_proxies = Vec::new();
    let mut out_groups = Vec::new();
    let mut out_rules = Vec::new();
    let mut kept: HashMap<ProxyKey, String> = HashMap::new();
    let mut taken: HashSet<String> = HashSet::new();
    let mut groups_by_name: HashMap<String, usize> = HashMap::new();
    let mut rendered_rules = HashSet::new();
    let mut stats = MergeStats::default();

    for (source, doc) in docs {
        let mapping = doc.as_mapping().ok_or_else(|| {
            format!("invalid config in profile {source}: expected a YAML mapping")
        })?;
        let mut name_map = HashMap::new();

        if let Some(Value::Sequence(proxies)) = mapping.get(key("proxies")) {
            for proxy in proxies {
                let mut proxy = proxy.clone();
                if !proxy.is_mapping() {
                    return Err(format!(
                        "invalid proxy in profile {source}: expected a YAML mapping"
                    )
                    .into());
                }
                let original_name = field(&proxy, "name");
                if original_name.is_empty() {
                    return Err(format!("invalid proxy in profile {source}: missing name").into());
                }
                let proxy_key = (
                    field(&proxy, "type"),
                    field(&proxy, "server"),
                    field(&proxy, "port"),
                    field(&proxy, "password"),
                );
                if let Some(winner) = kept.get(&proxy_key) {
                    stats.dropped += 1;
                    name_map.insert(original_name, winner.clone());
                    continue;
                }

                let out_name = unique_proxy_name(&original_name, source, &taken);
                proxy
                    .as_mapping_mut()
                    .unwrap()
                    .insert(key("name"), Value::String(out_name.clone()));
                out_proxies.push(proxy);
                taken.insert(out_name.clone());
                kept.insert(proxy_key, out_name.clone());
                name_map.insert(original_name, out_name);
            }
        }

        if let Some(Value::Sequence(groups)) = mapping.get(key("proxy-groups")) {
            for group in groups {
                let mut group = group.clone();
                if !group.is_mapping() {
                    return Err(format!(
                        "invalid proxy-group in profile {source}: expected a YAML mapping"
                    )
                    .into());
                }
                let group_name = field(&group, "name");
                if group_name.is_empty() {
                    return Err(
                        format!("invalid proxy-group in profile {source}: missing name").into(),
                    );
                }
                if let Some(Value::Sequence(members)) =
                    group.as_mapping_mut().unwrap().get_mut(key("proxies"))
                {
                    for member in members {
                        if let Value::String(name) = member
                            && let Some(mapped) = name_map.get(name)
                        {
                            *name = mapped.clone();
                        }
                    }
                }

                if let Some(existing_index) = groups_by_name.get(&group_name).copied() {
                    merge_group_members(&mut out_groups[existing_index], &group);
                    if field(&out_groups[existing_index], "type") != field(&group, "type") {
                        warn!(group = %group_name, "merged proxy-group with differing type");
                    }
                } else {
                    groups_by_name.insert(group_name, out_groups.len());
                    out_groups.push(group);
                }
            }
        }

        if let Some(Value::Sequence(rules)) = mapping.get(key("rules")) {
            for rule in rules {
                let rendered = serde_yaml::to_string(rule)?;
                if rendered_rules.insert(rendered) {
                    out_rules.push(rule.clone());
                }
            }
        }

        for (top_key, top_value) in mapping {
            if top_key == &key("proxies")
                || top_key == &key("proxy-groups")
                || top_key == &key("rules")
            {
                continue;
            }
            if out.contains_key(top_key) {
                warn!(key = ?top_key, profile = %source, "dropping conflicting top-level key");
            } else {
                out.insert(top_key.clone(), top_value.clone());
            }
        }
    }

    stats.proxies = out_proxies.len();
    stats.groups = out_groups.len();
    stats.rules = out_rules.len();
    out.insert(key("proxies"), Value::Sequence(out_proxies));
    out.insert(key("proxy-groups"), Value::Sequence(out_groups));
    out.insert(key("rules"), Value::Sequence(out_rules));
    Ok((Value::Mapping(out), stats))
}

pub fn merge(sources: &[String], out: Option<&str>) -> Result<MergeResult, Box<dyn Error>> {
    if sources.len() < 2 {
        return Err("merge needs at least two profiles".into());
    }
    let mut seen = HashSet::new();
    for source in sources {
        if !seen.insert(source) {
            return Err(format!("duplicate source: {source}").into());
        }
    }

    let name = out.map(str::to_string).unwrap_or_else(|| sources.join("+"));
    paths::validate_component(&name, "profile name")?;
    if sources.iter().any(|source| source == &name) {
        return Err(format!("cannot merge into one of its sources: {name}").into());
    }

    let dir = paths::profile_dir(&name);
    let created_at = if dir.exists() {
        let meta = crud::read_meta(&name)?;
        if meta.kind != ProfileKind::Merge {
            return Err(format!("profile {name} already exists and is not a merge output").into());
        }
        meta.created_at
    } else {
        crud::now_secs()
    };

    let docs: Result<Vec<_>, Box<dyn Error>> = sources
        .iter()
        .map(|source| {
            let raw = crud::read_raw(source)?;
            let value = serde_yaml::from_str(&raw)
                .map_err(|e| format!("invalid YAML in profile {source}: {e}"))?;
            Ok((source.clone(), value))
        })
        .collect();
    let docs = docs?;
    let (doc, stats) = merge_documents(&docs)?;
    crud::write_atomic(
        &paths::profile_raw_file(&name),
        &serde_yaml::to_string(&doc)?,
    )?;
    let meta = ProfileMeta {
        kind: ProfileKind::Merge,
        url: None,
        user_agent: None,
        sources: sources.to_vec(),
        created_at,
        updated_at: Some(crud::now_secs()),
    };
    crud::write_meta(&name, &meta)?;
    crud::refresh_active_config(&name)?;
    info!(name = %name, proxies = stats.proxies, dropped = stats.dropped, "merge complete");
    Ok(MergeResult { name, stats })
}

fn key(key: &str) -> Value {
    Value::String(key.to_string())
}

fn field(node: &Value, key_name: &str) -> String {
    node.as_mapping()
        .and_then(|map| map.get(key(key_name)))
        .map(|value| match value {
            Value::String(s) => s.to_owned(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default()
}

fn unique_proxy_name(name: &str, source: &str, taken: &HashSet<String>) -> String {
    if !taken.contains(name) {
        return name.to_string();
    }
    let first = format!("{name} ({source})");
    if !taken.contains(&first) {
        return first;
    }
    for n in 2.. {
        let candidate = format!("{name} ({source} {n})");
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!()
}

fn merge_group_members(existing: &mut Value, later: &Value) {
    let Some(existing_members) = existing
        .as_mapping_mut()
        .and_then(|map| map.get_mut(key("proxies")))
        .and_then(Value::as_sequence_mut)
    else {
        return;
    };
    let Some(later_members) = later
        .as_mapping()
        .and_then(|map| map.get(key("proxies")))
        .and_then(Value::as_sequence)
    else {
        return;
    };
    let mut existing_set: HashSet<String> = existing_members.iter().map(render_member).collect();
    for member in later_members {
        if existing_set.insert(render_member(member)) {
            existing_members.push(member.clone());
        }
    }
}

fn render_member(member: &Value) -> String {
    serde_yaml::to_string(member).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(name: &str, yaml: &str) -> (String, Value) {
        (name.to_string(), serde_yaml::from_str(yaml).unwrap())
    }

    fn seq<'a>(value: &'a Value, key_name: &str) -> &'a Vec<Value> {
        value
            .as_mapping()
            .unwrap()
            .get(key(key_name))
            .unwrap()
            .as_sequence()
            .unwrap()
    }

    #[test]
    fn test_duplicate_proxy_identity_collapses() {
        let docs = vec![
            doc(
                "a",
                "proxies:\n  - {name: a1, type: ss, server: 1.1.1.1, port: 443, password: pw}\n",
            ),
            doc(
                "b",
                "proxies:\n  - {name: b1, type: ss, server: 1.1.1.1, port: '443', password: pw}\n",
            ),
        ];
        let (merged, stats) = merge_documents(&docs).unwrap();
        assert_eq!(seq(&merged, "proxies").len(), 1);
        assert_eq!(stats.dropped, 1);
    }

    #[test]
    fn test_same_name_different_server_is_renamed() {
        let docs = vec![
            doc(
                "a",
                "proxies:\n  - {name: node1, type: ss, server: 1.1.1.1, port: 443, password: pw1}\n",
            ),
            doc(
                "b",
                "proxies:\n  - {name: node1, type: ss, server: 2.2.2.2, port: 443, password: pw2}\n",
            ),
        ];
        let (merged, _) = merge_documents(&docs).unwrap();
        assert_eq!(field(&seq(&merged, "proxies")[1], "name"), "node1 (b)");
    }

    #[test]
    fn test_group_members_union_in_order() {
        let docs = vec![
            doc(
                "a",
                "proxies:\n  - {name: a1, type: ss, server: 1.1.1.1, port: 443}\nproxy-groups:\n  - {name: PROXY, type: select, proxies: [a1]}\n",
            ),
            doc(
                "b",
                "proxies:\n  - {name: b1, type: ss, server: 2.2.2.2, port: 443}\nproxy-groups:\n  - {name: PROXY, type: select, proxies: [b1, DIRECT]}\n",
            ),
        ];
        let (merged, _) = merge_documents(&docs).unwrap();
        let members = seq(&seq(&merged, "proxy-groups")[0], "proxies");
        assert_eq!(
            members,
            &vec![
                Value::String("a1".into()),
                Value::String("b1".into()),
                Value::String("DIRECT".into())
            ]
        );
    }

    #[test]
    fn test_group_member_duplicate_rewritten_to_winner() {
        let docs = vec![
            doc(
                "a",
                "proxies:\n  - {name: a1, type: ss, server: 1.1.1.1, port: 443, password: pw}\nproxy-groups:\n  - {name: PROXY, type: select, proxies: [a1]}\n",
            ),
            doc(
                "b",
                "proxies:\n  - {name: b1, type: ss, server: 1.1.1.1, port: 443, password: pw}\nproxy-groups:\n  - {name: PROXY, type: select, proxies: [b1]}\n",
            ),
        ];
        let (merged, _) = merge_documents(&docs).unwrap();
        let members = seq(&seq(&merged, "proxy-groups")[0], "proxies");
        assert_eq!(members, &vec![Value::String("a1".into())]);
    }

    #[test]
    fn test_duplicate_rules_once_order_preserved() {
        let docs = vec![
            doc(
                "a",
                "rules:\n  - MATCH,PROXY\n  - DOMAIN,example.com,DIRECT\n",
            ),
            doc(
                "b",
                "rules:\n  - MATCH,PROXY\n  - DOMAIN,example.org,DIRECT\n",
            ),
        ];
        let (merged, _) = merge_documents(&docs).unwrap();
        let rules = seq(&merged, "rules");
        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0], Value::String("MATCH,PROXY".into()));
        assert_eq!(rules[2], Value::String("DOMAIN,example.org,DIRECT".into()));
    }

    #[test]
    fn test_top_level_scalar_first_wins() {
        let docs = vec![
            doc("a", "mixed-port: 7890\n"),
            doc("b", "mixed-port: 1080\n"),
        ];
        let (merged, _) = merge_documents(&docs).unwrap();
        assert_eq!(field(&merged, "mixed-port"), "7890");
    }

    #[test]
    fn test_sequence_root_errors() {
        let docs = vec![doc("a", "- nope\n")];
        let err = merge_documents(&docs).unwrap_err();
        assert!(err.to_string().contains("expected a YAML mapping"));
    }
}
