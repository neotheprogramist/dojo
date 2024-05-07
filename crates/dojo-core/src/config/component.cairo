mod errors {
    const INVALID_CALLER: felt252 = 'Config: not owner or operator';
    const ALREADY_REGISTERED: felt252 = 'Config: already operator';
    const NOT_OPERATOR: felt252 = 'Config: not operator';
}

#[starknet::component]
mod Config {
    use dojo::config::interface::IConfig;
    use starknet::ContractAddress;
    use super::errors;
    use starknet::get_caller_address;
    use starknet::event::EventEmitter;

    #[event]
    #[derive(Drop, starknet::Event, Debug, PartialEq)]
    pub enum Event {
        ProgramHashUpdate: ProgramHashUpdate,
        FactsRegistryUpdate: FactsRegistryUpdate
    }

    #[derive(Drop, starknet::Event, Debug, PartialEq)]
    pub struct ProgramHashUpdate {
        program_hash: felt252,
        of_merger_program: bool
    }

    #[derive(Drop, starknet::Event, Debug, PartialEq)]
    pub struct FactsRegistryUpdate {
        address: ContractAddress
    }

    #[storage]
    struct Storage {
        program_hash: felt252,
        merger_program_hash: felt252,
        facts_registry: ContractAddress,
        owner: ContractAddress
    }

    #[generate_trait]
    impl InternalImpl<
        TContractState, +HasComponent<TContractState>
    > of InternalTrait<TContractState> {
        fn initializer(ref self: ComponentState<TContractState>, owner: ContractAddress) {
            self.owner.write(owner);
        }
    }


    #[embeddable_as(ConfigImpl)]
    impl Config<
        TContractState, +HasComponent<TContractState>
    > of IConfig<ComponentState<TContractState>> {
        fn set_program_hash(ref self: ComponentState<TContractState>, program_hash: felt252, for_merger_program: bool) {
            assert(get_caller_address() == self.owner.read(), errors::INVALID_CALLER);
            if (for_merger_program) {
                self.merger_program_hash.write(program_hash);
            } else {
                self.program_hash.write(program_hash);
            }
            self.emit(ProgramHashUpdate { program_hash: program_hash, of_merger_program: for_merger_program});
        }

        fn get_program_hash(self: @ComponentState<TContractState>, of_merger_program: bool) -> felt252 {
            if (of_merger_program) {
                self.merger_program_hash.read()
            } else {
                self.program_hash.read()
            }
        }

        fn set_facts_registry(ref self: ComponentState<TContractState>, address: ContractAddress) {
            assert(get_caller_address() == self.owner.read(), errors::INVALID_CALLER);
            self.facts_registry.write(address);
            self.emit(FactsRegistryUpdate { address: address });
        }

        fn get_facts_registry(self: @ComponentState<TContractState>) -> ContractAddress {
            self.facts_registry.read()
        }
    }
}

