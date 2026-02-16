pub struct MapMode;

impl MapMode {
    const PROXY_NAME_SUFFIX: &str = "__slider_proxy";

    pub fn proxy_name(name: &str) -> String {
        format!("{}{}", name, Self::PROXY_NAME_SUFFIX)
    }

    pub fn unproxied_name(proxy_name: &str) -> Option<String> {
        proxy_name
            .strip_suffix(Self::PROXY_NAME_SUFFIX)
            .map(|s| s.to_string())
    }

    pub fn is_proxy_name(name: &str) -> bool {
        name.ends_with(Self::PROXY_NAME_SUFFIX)
    }
}
