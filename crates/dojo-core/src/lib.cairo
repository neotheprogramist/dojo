mod base;
#[cfg(test)]
mod base_test;
mod database;
mod interfaces;
#[cfg(test)]
mod database_test;
mod model;
mod packing;
#[cfg(test)]
mod packing_test;
mod world;
#[cfg(test)]
mod world_test;

#[cfg(test)]
mod test_utils;

#[cfg(test)]
mod benchmarks;

mod components;
mod resource_metadata;


// Components 
mod config {
    mod component;
    mod interface;
    mod mock;

    use component::config_cpt;
    use interface::{IConfig, IConfigDispatcher, IConfigDispatcherTrait};
    use mock::config_mock;

    #[cfg(test)]
    mod tests {
        mod test_config;
    }
}
