#[derive(serde::Serialize, serde::Deserialize)]
pub struct Info {
    pub path: String,
    pub size: u64,
    pub owner: String,
}
