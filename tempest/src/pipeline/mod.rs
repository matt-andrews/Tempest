pub mod test_capabilities;
pub mod assert_capabilities;

use crate::engine::test_capabilities::{get_test_capability, TestCapability};
use crate::models::directory_model::DirectoryModel;
use crate::models::options_model::OptionsModel;

pub async fn execute(discovered: DirectoryModel, default_options: OptionsModel) -> anyhow::Result<()>{
    for directory in discovered.walk() {
        let base_options = default_options
            .clone()
            .merge(directory.options.iter()
                .cloned()
                .reduce(|acc, next| acc.merge(next))
                .unwrap_or_default()
            );

        for descriptor in directory.files.iter().flat_map(|m| m.descendants()) {

            let mut options = descriptor.options.clone().unwrap_or_default();
            options = base_options.clone().merge(options);

            if let Some(capability) = get_test_capability(descriptor, &options){
                let test_result = capability.test().await;
            }
        }
    }

    Ok(())
}