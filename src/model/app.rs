use chrono;

typed_id!(UserId);
typed_id!(AppId);

#[derive(Debug)]
pub struct User {
    pub user_id: UserId,
    pub email: String,
    pub tz: Option<chrono::offset::FixedOffset>,

}

#[derive(Serialize, Deserialize, Debug)]
pub struct Application {
    pub app_id: AppId,
    pub app_secret: Option<String>,
}
