//! A compose file's services and the ports each publishes, read by indent.
//!
//! **Read line by line, the way `drifting::declared` reads TOML**, rather than
//! parsed into an untyped document outside the two crates bytes enter through.
//! The price is stated: any shape this does not recognise makes the whole
//! file `None`, which the caller reports as not read. A compose file half read
//! would name some services and silently drop the rest.
//!
//! **The container port is the evidence.** In `"8080:3000"` the published side
//! is what Armada leases per worktree and never writes down; the container
//! side is what a Manifest declares.

use super::workspaces::scalar;

/// One service, and each entry under its `ports`.
pub(super) struct Service {
    pub(super) name: String,
    pub(super) ports: Vec<Port>,
}

/// One `ports` entry: its key in the file, and its container port or why
/// there is none to read.
pub(super) struct Port {
    pub(super) key: String,
    pub(super) container: Result<u16, String>,
}

/// Every service under a top-level `services:`, or `None` where the file is in
/// a shape this does not read. No `services:` at all is no services.
pub(super) fn services(text: &str) -> Option<Vec<Service>> {
    let lines: Vec<(usize, &str)> = text
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .map(|line| (line.len() - line.trim_start_matches(' ').len(), line.trim()))
        .collect();
    if lines.iter().any(|(_, line)| line.starts_with('\t')) {
        return None;
    }
    let Some(start) = lines.iter().position(|(indent, line)| {
        *indent == 0 && line.split('#').next().map(str::trim) == Some("services:")
    }) else {
        return match lines
            .iter()
            .any(|(i, l)| *i == 0 && l.starts_with("services:"))
        {
            // `services: {}` and its kin — declared, and empty or flow-styled.
            true => lines
                .iter()
                .any(|(i, l)| *i == 0 && l.replace(' ', "") == "services:{}")
                .then(Vec::new),
            false => Some(Vec::new()),
        };
    };
    let body: Vec<(usize, &str)> = lines[start + 1..]
        .iter()
        .take_while(|(indent, _)| *indent > 0)
        .copied()
        .collect();
    let service_indent = body.first()?.0;

    let mut services: Vec<Service> = Vec::new();
    let mut property_indent = None;
    let mut in_ports: Option<usize> = None;
    for (indent, line) in body {
        if indent == service_indent {
            let name = line.strip_suffix(':').or_else(|| {
                let (name, rest) = line.split_once(':')?;
                rest.trim().starts_with('&').then_some(name)
            })?;
            services.push(Service {
                name: name.trim().to_string(),
                ports: Vec::new(),
            });
            property_indent = None;
            in_ports = None;
            continue;
        }
        if indent < service_indent {
            return None;
        }
        let service = services.last_mut()?;
        let property = *property_indent.get_or_insert(indent);
        if indent == property {
            in_ports = None;
            if let Some(rest) = line.strip_prefix("ports:") {
                let rest = rest.trim();
                if rest.starts_with('[') {
                    let entries = crate::drifting::declared::quoted(rest)?;
                    for entry in entries {
                        let key = port_key(service);
                        service.ports.push(Port {
                            key,
                            container: short(&entry),
                        });
                    }
                } else if rest.is_empty() {
                    in_ports = Some(indent);
                } else {
                    return None;
                }
            }
            continue;
        }
        if in_ports.is_none() {
            continue;
        }
        if let Some(item) = line.strip_prefix("- ") {
            let item = item.trim();
            match item.split_once(':') {
                // The long form: `- target: 80`, other keys beside it.
                Some((field, value)) if !item.starts_with(['"', '\'']) && is_field(field) => {
                    let key = port_key(service);
                    let container = match field.trim() {
                        "target" => number(value),
                        _ => Err("a long-form entry whose first key is not its target".into()),
                    };
                    service.ports.push(Port { key, container });
                }
                _ => {
                    let key = port_key(service);
                    let container = scalar(item)
                        .ok_or_else(|| "an entry this read does not follow".to_string())
                        .and_then(|entry| short(&entry));
                    service.ports.push(Port { key, container });
                }
            }
        } else if let Some(value) = line.strip_prefix("target:") {
            // `target` after another key of the same long-form entry.
            if let Some(last) = service.ports.last_mut() {
                last.container = number(value);
            }
        }
    }
    Some(services)
}

fn port_key(service: &Service) -> String {
    format!("services.{}.ports[{}]", service.name, service.ports.len())
}

/// A long-form key, as opposed to the `:` inside `"8080:3000"`.
fn is_field(field: &str) -> bool {
    let field = field.trim();
    !field.is_empty() && field.chars().all(|c| c.is_ascii_lowercase() || c == '_')
}

fn number(value: &str) -> Result<u16, String> {
    let value = value.trim().trim_matches(['"', '\'']);
    value
        .parse()
        .map_err(|_| format!("`{value}` is not a port number"))
}

/// The container side of a short-form entry: `3000`, `8080:3000`,
/// `127.0.0.1:8080:3000/tcp`.
fn short(entry: &str) -> Result<u16, String> {
    let entry = entry.trim();
    let entry = entry.split('/').next().unwrap_or(entry);
    let container = entry.rsplit(':').next().unwrap_or(entry);
    if container.contains('-') {
        return Err(format!(
            "`{entry}` is a range, which a Manifest port is not"
        ));
    }
    if container.contains('$') {
        return Err(format!("`{entry}` is filled in from the environment"));
    }
    number(container)
}
