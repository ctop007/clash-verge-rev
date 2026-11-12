use anyhow::{Context as _, Result, bail, ensure};
use base64::{Engine as _, engine::general_purpose};
use percent_encoding::percent_decode_str;
use serde_json::{Value, json};
use serde_yaml_ng::Mapping;
use std::collections::{HashMap, HashSet};
use tauri::Url;

pub(super) fn normalize(data: &str) -> Result<String> {
    if let Ok(yaml) = serde_yaml_ng::from_str::<Mapping>(data) {
        ensure!(
            yaml.contains_key("proxies") || yaml.contains_key("proxy-providers"),
            "profile does not contain `proxies` or `proxy-providers`"
        );
        return Ok(data.to_owned());
    }

    let decoded;
    let links = if ["vless://", "hysteria2://", "hy2://"]
        .iter()
        .any(|scheme| data.trim_start().starts_with(scheme))
    {
        data
    } else {
        let encoded: String = data.chars().filter(|c| !c.is_whitespace()).collect();
        let bytes = [
            general_purpose::STANDARD,
            general_purpose::STANDARD_NO_PAD,
            general_purpose::URL_SAFE,
            general_purpose::URL_SAFE_NO_PAD,
        ]
        .iter()
        .find_map(|engine| engine.decode(&encoded).ok())
        .context("expected Clash YAML or a Base64 VLESS/Hysteria2 subscription")?;
        decoded = String::from_utf8(bytes).context("subscription is not UTF-8")?;
        &decoded
    };

    let mut proxies = Vec::new();
    let mut names = Vec::new();
    let mut used = HashSet::from(["PROXY".to_owned(), "DIRECT".to_owned(), "REJECT".to_owned()]);
    for (index, line) in links.lines().map(str::trim).filter(|line| !line.is_empty()).enumerate() {
        let mut proxy = parse_uri(line).with_context(|| format!("invalid subscription node {}", index + 1))?;
        let original = proxy["name"].as_str().context("missing node name")?;
        let mut name = original.to_owned();
        let mut suffix = 2;
        while !used.insert(name.clone()) {
            name = format!("{original} ({suffix})");
            suffix += 1;
        }
        proxy["name"] = json!(name);
        names.push(name);
        proxies.push(proxy);
    }
    ensure!(!proxies.is_empty(), "subscription contains no nodes");
    serde_yaml_ng::to_string(&json!({
        "proxies": proxies,
        "proxy-groups": [{"name": "PROXY", "type": "select", "proxies": names}],
        "rules": ["MATCH,PROXY"]
    }))
    .context("failed to serialize subscription")
}

fn decode(value: &str) -> Result<String> {
    percent_decode_str(value)
        .decode_utf8()
        .map(|value| value.into_owned())
        .context("invalid UTF-8 in node URI")
}

fn parse_uri(line: &str) -> Result<Value> {
    let url = Url::parse(line).context("invalid node URI")?;
    let is_vless = url.scheme() == "vless";
    ensure!(
        is_vless || matches!(url.scheme(), "hysteria2" | "hy2"),
        "unsupported node protocol; expected VLESS or Hysteria2"
    );
    let server = url
        .host_str()
        .context("missing node server")?
        .trim_start_matches('[')
        .trim_end_matches(']');
    let port = url
        .port()
        .or(if is_vless { None } else { Some(443) })
        .context("missing node port")?;
    ensure!(port > 0, "invalid node port");
    let auth = if let Some(password) = url.password() {
        format!("{}:{}", decode(url.username())?, decode(password)?)
    } else {
        decode(url.username())?
    };
    ensure!(!auth.is_empty(), "missing node credentials");
    let name = decode(url.fragment().unwrap_or_default())?;
    let name = if name.trim().is_empty() {
        format!("{} {server}:{port}", url.scheme())
    } else {
        name
    };
    let params: HashMap<_, _> = url.query_pairs().collect();
    let param = |key: &str| {
        params
            .get(key)
            .map(|value| value.as_ref())
            .filter(|value| !value.is_empty())
    };
    let mut proxy = json!({
        "name": name, "type": if is_vless { "vless" } else { "hysteria2" },
        "server": server, "port": port
    });
    proxy[if is_vless { "uuid" } else { "password" }] = json!(auth);
    if let Some(sni) = param("sni").or_else(|| param("peer")) {
        proxy[if is_vless { "servername" } else { "sni" }] = json!(sni);
    }
    if let Some(alpn) = param("alpn") {
        proxy["alpn"] = json!(alpn.split(',').collect::<Vec<_>>());
    }
    if let Some(value) = param("allowInsecure").or_else(|| param("insecure")) {
        proxy["skip-cert-verify"] = json!(matches!(value, "1" | "true"));
    }

    if is_vless {
        ensure!(
            matches!(param("encryption"), None | Some("none")),
            "unsupported VLESS encryption"
        );
        let security = param("security").unwrap_or("none");
        ensure!(
            matches!(security, "none" | "tls" | "reality"),
            "unsupported VLESS security"
        );
        proxy["tls"] = json!(security != "none");
        if let Some(flow) = param("flow") {
            proxy["flow"] = json!(flow);
        }
        if let Some(fingerprint) = param("fp") {
            proxy["client-fingerprint"] = json!(fingerprint);
        }
        if security == "reality" {
            let public_key = param("pbk").context("missing Reality public key")?;
            proxy["reality-opts"] = json!({"public-key": public_key});
            if let Some(short_id) = param("sid") {
                proxy["reality-opts"]["short-id"] = json!(short_id);
            }
        }
        match param("type").unwrap_or("tcp") {
            "tcp" => {}
            "ws" => {
                proxy["network"] = json!("ws");
                proxy["ws-opts"] = json!({"path": param("path").unwrap_or("/")});
                if let Some(host) = param("host") {
                    proxy["ws-opts"]["headers"] = json!({"Host": host});
                }
            }
            _ => bail!("unsupported VLESS transport; expected TCP or WebSocket"),
        }
        ensure!(
            param("headerType").is_none_or(|value| value == "none"),
            "unsupported VLESS header type"
        );
    } else {
        if let Some(obfs) = param("obfs").filter(|value| *value != "none") {
            proxy["obfs"] = json!(obfs);
            if let Some(password) = param("obfs-password") {
                proxy["obfs-password"] = json!(password);
            }
        }
        if let Some(ports) = param("mport") {
            proxy["ports"] = json!(ports);
        }
        if let Some(fingerprint) = param("pinSHA256") {
            proxy["fingerprint"] = json!(fingerprint);
        }
    }
    Ok(proxy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_base64_subscription_without_losing_transport_options() -> Result<()> {
        let links = "vless://00000000-0000-0000-0000-000000000001@one.example:443?security=reality&type=tcp&pbk=key&sid=ab&fp=chrome&sni=tls.example&flow=xtls-rprx-vision#%E9%A6%99%E6%B8%AF\n\
                     vless://00000000-0000-0000-0000-000000000002@two.example:443?security=tls&type=ws&host=ws.example&path=%2Fsocket%3Fed%3D2048&alpn=h2,http%2F1.1#PROXY\n\
                     hysteria2://user%3Apass@[::1]:443?sni=tls.example&mport=443,8443&obfs=salamander&obfs-password=secret#PROXY";
        for engine in [general_purpose::STANDARD, general_purpose::URL_SAFE_NO_PAD] {
            let encoded = engine.encode(links);
            let config: Value = serde_yaml_ng::from_str(&normalize(&format!("{encoded}\n"))?)?;
            let proxies = &config["proxies"];
            assert_eq!(proxies[0]["name"], "香港");
            assert_eq!(proxies[0]["reality-opts"]["public-key"], "key");
            assert_eq!(proxies[0]["reality-opts"]["short-id"], "ab");
            assert_eq!(proxies[0]["flow"], "xtls-rprx-vision");
            assert_eq!(proxies[0]["client-fingerprint"], "chrome");
            assert_eq!(proxies[1]["ws-opts"]["path"], "/socket?ed=2048");
            assert_eq!(proxies[1]["ws-opts"]["headers"]["Host"], "ws.example");
            assert_eq!(proxies[1]["alpn"], json!(["h2", "http/1.1"]));
            assert_eq!(proxies[2]["password"], "user:pass");
            assert_eq!(proxies[2]["server"], "::1");
            assert_eq!(proxies[2]["ports"], "443,8443");
            assert_eq!(proxies[2]["obfs-password"], "secret");
            assert_eq!(
                config["proxy-groups"][0]["proxies"],
                json!(["香港", "PROXY (2)", "PROXY (3)"])
            );
            assert_eq!(normalize(links)?, normalize(&encoded)?);
        }
        Ok(())
    }

    #[test]
    fn preserves_yaml_and_rejects_partial_or_invalid_subscriptions() -> Result<()> {
        let yaml = "# keep comments\nproxies: []\nrules: [MATCH,DIRECT]\n";
        assert_eq!(normalize(yaml)?, yaml);
        let provider = "proxy-providers: {}\n";
        assert_eq!(normalize(provider)?, provider);
        assert!(normalize("rules: []").is_err());
        assert!(normalize("").is_err());
        let invalid =
            general_purpose::STANDARD.encode("vless://secret@one.example:443\nunsupported://secret@two.example:443");
        let error = normalize(&invalid).err().context("unsupported node was accepted")?;
        let message = format!("{error:#}");
        assert!(message.contains("node 2"));
        assert!(!message.contains("secret"));
        assert!(!message.contains("two.example"));
        Ok(())
    }
}
