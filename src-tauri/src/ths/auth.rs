use anyhow::{anyhow, Context, Result};
use base64::Engine;
use regex::Regex;

const AUTH_BASE: &str = "https://auth.10jqka.com.cn/verify2";
const DO_COOKIE: &str = "https://upass.10jqka.com.cn/docookie2.php";
const TA_APP_ID: &str = "2022021114090152";
const RSA_VERSION_FALLBACK: &str = "default_5";
const AUTH_UA: &str = "同花顺/7.0.10 CFNetwork/1333.0.4 Darwin/21.5.0";

pub struct ThsSession {
    pub cookies: String,
}

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

/// XML 属性值反转义（PEM 公钥里的换行等）
fn xml_unescape(s: &str) -> String {
    s.replace("&#10;", "\n")
        .replace("&#13;", "\r")
        .replace("&#9;", "\t")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// 从 XML 里取 <item .../> 的属性
fn item_attr(xml: &str, attr: &str) -> Option<String> {
    let re = Regex::new(&format!(r#"<item\b[^>]*\b{attr}="([^"]*)""#)).ok()?;
    re.captures(xml).map(|c| xml_unescape(&c[1]))
}

fn rsa_encrypt_pem(pubkey_pem: &str, plain: &str) -> Result<String> {
    use rsa::pkcs1v15::Pkcs1v15Encrypt;
    use rsa::pkcs8::DecodePublicKey;
    use rsa::RsaPublicKey;
    let key = RsaPublicKey::from_public_key_pem(pubkey_pem)
        .map_err(|e| anyhow!("公钥解析失败: {e}"))?;
    let mut rng = rand::thread_rng();
    let encrypted = key
        .encrypt(&mut rng, Pkcs1v15Encrypt, plain.as_bytes())
        .map_err(|e| anyhow!("RSA 加密失败: {e}"))?;
    Ok(b64().encode(encrypted))
}

/// 账号密码登录，返回整包 cookie 字符串。
/// 流程：get_pubkey → unified_login → mainverify(取 signvalid) → docookie2。
pub async fn login(client: &reqwest::Client, account: &str, passwd: &str) -> Result<ThsSession> {
    // 1. RSA 公钥
    let xml = client
        .get(AUTH_BASE)
        .query(&[("reqtype", "do_rsa"), ("type", "get_pubkey")])
        .header("User-Agent", AUTH_UA)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .context("获取公钥失败")?
        .text()
        .await?;
    let pubkey = item_attr(&xml, "pubkey").ok_or_else(|| anyhow!("公钥响应异常: {xml}"))?;
    let rsa_version =
        item_attr(&xml, "rsa_version").unwrap_or_else(|| RSA_VERSION_FALLBACK.to_string());

    // 2. unified_login
    let enc_account = rsa_encrypt_pem(&pubkey, account)?;
    let enc_passwd = rsa_encrypt_pem(&pubkey, passwd)?;
    let xml = client
        .get(AUTH_BASE)
        .query(&[
            ("reqtype", "unified_login"),
            ("account", enc_account.as_str()),
            ("passwd", enc_passwd.as_str()),
            ("msg", "1"),
            ("rsa_version", rsa_version.as_str()),
            ("ta_appid", TA_APP_ID),
        ])
        .header("User-Agent", AUTH_UA)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .context("登录请求失败")?
        .text()
        .await?;
    let userid = item_attr(&xml, "userid").ok_or_else(|| anyhow!("登录失败（账号或密码错误）: {}", brief_xml(&xml)))?;
    let sessionid = item_attr(&xml, "sessionid")
        .ok_or_else(|| anyhow!("登录响应缺少 sessionid: {}", brief_xml(&xml)))?;

    // 3. mainverify → signvalid
    let xml = client
        .get(AUTH_BASE)
        .query(&[
            ("reqtype", "mainverify"),
            ("userid", userid.as_str()),
            ("sessionid", sessionid.as_str()),
            ("qsid", "8003"),
            ("product", "S01"),
            ("version", "11.4.1.3"),
            ("imei", "ZjI6MDY6NGE6NzI6MjQ6NTA="),
            ("sdsn", ""),
            ("rsa_version", rsa_version.as_str()),
            ("nohqlist", "0"),
            ("securities", "%E5%90%8C%E8%8A%B1%E9%A1%BA%E8%BF%9C%E8%88%AA%E7%89%88"),
        ])
        .header("User-Agent", AUTH_UA)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .context("mainverify 失败")?
        .text()
        .await?;
    let passport = item_attr(&xml, "passport").ok_or_else(|| anyhow!("mainverify 缺少 passport: {}", brief_xml(&xml)))?;
    let signvalid = passport
        .split('|')
        .find_map(|seg| {
            let (k, v) = seg.split_once('=')?;
            if k.trim() == "signvalid" { Some(v.trim().to_string()) } else { None }
        })
        .ok_or_else(|| anyhow!("passport 中无 signvalid"))?;

    // 4. 兑换整包 cookie
    let resp = client
        .get(DO_COOKIE)
        .query(&[
            ("userid", userid.as_str()),
            ("sessionid", sessionid.as_str()),
            ("signvalid", signvalid.as_str()),
        ])
        .header("User-Agent", AUTH_UA)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .context("cookie 兑换失败")?;
    let mut pairs: Vec<String> = Vec::new();
    for value in resp.headers().get_all(reqwest::header::SET_COOKIE) {
        if let Ok(s) = value.to_str() {
            if let Some(kv) = s.split(';').next() {
                let kv = kv.trim().to_string();
                if !kv.is_empty() && !pairs.iter().any(|p| p == &kv) {
                    pairs.push(kv);
                }
            }
        }
    }
    if pairs.is_empty() {
        anyhow::bail!("cookie 兑换响应为空（登录可能被风控拦截）");
    }
    let cookies = pairs.join("; ");
    Ok(ThsSession { cookies })
}

fn brief_xml(xml: &str) -> String {
    let re = Regex::new(r"<error[^>]*>").ok();
    if xml.chars().count() > 200 {
        let cut: String = xml.chars().take(200).collect();
        if let Some(re) = re {
            if let Some(c) = re.captures(&cut) {
                return c[0].to_string();
            }
        }
        return format!("{cut}...");
    }
    xml.to_string()
}
