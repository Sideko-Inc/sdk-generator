use sideko_rest_api::models::Api;

use crate::utils::url_builder::ApiUrl;

pub struct TabledApi {
    pub api: Api,
    pub subdomain: String,
}
impl tabled::Tabled for TabledApi {
    const LENGTH: usize = 5;

    fn fields(&self) -> Vec<std::borrow::Cow<'_, str>> {
        vec![
            self.api.name.as_str().into(),
            self.api.version_count.to_string().into(),
            ApiUrl::new(&self.api.name).build(&self.subdomain).into(),
            self.api.id.as_str().into(),
            self.api.created_at.as_str().into(),
        ]
    }

    fn headers() -> Vec<std::borrow::Cow<'static, str>> {
        vec![
            "Name".into(),
            "Versions".into(),
            "URL".into(),
            "ID".into(),
            "Created At".into(),
        ]
    }
}
