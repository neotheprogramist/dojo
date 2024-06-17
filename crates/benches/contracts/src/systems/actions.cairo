use benches::models::position::Position;
use benches::models::moves::Direction;
use benches::models::character::Abilities;

// define the interface
#[dojo::interface]
trait IActions {
    fn spawn(ref world: IWorldDispatcher);
    fn move(ref world: IWorldDispatcher, direction: Direction);

    fn bench_basic_emit(ref world: IWorldDispatcher, name: felt252);
    fn bench_basic_set(ref world: IWorldDispatcher, name: felt252);
    fn bench_basic_double_set(ref world: IWorldDispatcher, name: felt252);
    fn bench_basic_get(ref world: IWorldDispatcher);

    fn bench_primitive_pass_many(
        world: @IWorldDispatcher,
        first: felt252,
        second: felt252,
        third: felt252,
        fourth: felt252,
        fifth: felt252,
        sixth: felt252,
        seventh: felt252,
        eighth: felt252,
        ninth: felt252,
    );
    fn bench_primitive_iter(world: @IWorldDispatcher, n: u32);
    fn bench_primitive_hash(world: @IWorldDispatcher, a: felt252, b: felt252, c: felt252);

    fn bench_complex_set_default(ref world: IWorldDispatcher);
    fn bench_complex_set_with_smaller(ref world: IWorldDispatcher, abilities: Abilities);
    fn bench_complex_update_minimal(ref world: IWorldDispatcher, earned: u32);
    fn bench_complex_update_minimal_nested(ref world: IWorldDispatcher, which: u8);
    fn bench_complex_get(world: @IWorldDispatcher);
    fn bench_complex_get_minimal(world: @IWorldDispatcher) -> u32;
    fn bench_complex_check(world: @IWorldDispatcher, ability: felt252, threshold: u8) -> bool;

    fn is_prime(world: @IWorldDispatcher, n: felt252) -> bool;
}

// dojo decorator
#[dojo::contract]
mod actions {
    use super::IActions;

    use starknet::{ContractAddress, get_caller_address};
    use benches::models::{position::{Position, Vec2}, moves::{Moves, Direction}};
    use benches::models::character::{Character, Abilities, Stats, Weapon, Sword, Alias};
    use poseidon::poseidon_hash_span;

    // declaring custom event struct
    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        Moved: Moved,
        Aliased: Aliased,
    }

    // declaring custom event struct
    #[derive(Drop, starknet::Event)]
    struct Moved {
        player: ContractAddress,
        direction: Direction
    }

    #[derive(Drop, starknet::Event)]
    struct Aliased {
        player: ContractAddress,
        name: felt252,
    }

    fn next_position(mut position: Position, direction: Direction) -> Position {
        match direction {
            Direction::None => { return position; },
            Direction::Left => { position.vec.x -= 1; },
            Direction::Right => { position.vec.x += 1; },
            Direction::Up => { position.vec.y -= 1; },
            Direction::Down => { position.vec.y += 1; },
        };
        position
    }

    #[abi(embed_v0)]
    impl ActionsImpl of IActions<ContractState> {
        fn spawn(ref world: IWorldDispatcher) {
            let player = get_caller_address();
            let position = get!(world, player, (Position));
            let moves = get!(world, player, (Moves));

            set!(
                world,
                (
                    Moves { player, remaining: 1_000_000, last_direction: Direction::None },
                    Position { player, vec: Vec2 { x: 1_000_000, y: 1_000_000 } },
                )
            );
        }

        fn move(ref world: IWorldDispatcher, direction: Direction) {
            let player = get_caller_address();
            let (mut position, mut moves) = get!(world, player, (Position, Moves));
            moves.remaining -= 1;
            moves.last_direction = direction;
            let next = next_position(position, direction);

            set!(world, (moves, next));

            emit!(world, Moved { player, direction });
        }


        fn bench_basic_emit(ref world: IWorldDispatcher, name: felt252) {

            let player = get_caller_address();
            emit!(world, Aliased { player, name: name });
        }

        fn bench_basic_set(ref world: IWorldDispatcher, name: felt252) {
            let player = get_caller_address();

            set!(world, Alias { player, name: name });
        }

        fn bench_basic_double_set(ref world: IWorldDispatcher, name: felt252) {
            let player = get_caller_address();

            set!(world, Alias { player, name: name });
            set!(world, Alias { player, name: name });
        }

        fn bench_basic_get(ref world: IWorldDispatcher) {
            let player = get_caller_address();

            get!(world, player, Alias);
        }

        fn bench_primitive_pass_many(world: @IWorldDispatcher,
            first: felt252,
            second: felt252,
            third: felt252,
            fourth: felt252,
            fifth: felt252,
            sixth: felt252,
            seventh: felt252,
            eighth: felt252,
            ninth: felt252,
        ) {
            let sum = first + second + third + fourth + fifth + sixth + seventh + eighth + ninth;
        }

        fn bench_primitive_iter(world: @IWorldDispatcher, n: u32) {
            let mut i = 0;
            loop {
                if i == n {
                    break;
                }
                i += 1;
            }
        }

        fn bench_primitive_hash(world: @IWorldDispatcher, a: felt252, b: felt252, c: felt252) { 
            let hash = poseidon_hash_span(array![a, b, c].span());
        }

        
        fn bench_complex_set_default(ref world: IWorldDispatcher) {
            let caller = get_caller_address();

            set!(world, Character {
                caller: get_caller_address(),
                heigth: 170,
                abilities: Abilities {
                    strength: 8,
                    dexterity: 8,
                    constitution: 8,
                    intelligence: 8,
                    wisdom: 8,
                    charisma: 8,
                },
                stats: Stats {
                    kills: 0,
                    deaths: 0,
                    rests: 0,
                    hits: 0,
                    blocks: 0,
                    walked: 0,
                    runned: 0,
                    finished: false,
                    romances: 0,
                },
                weapon: Weapon::Fists((
                    Sword {
                        swordsmith: get_caller_address(),
                        damage: 10,
                    },
                    Sword {
                        swordsmith: get_caller_address(),
                        damage: 10,
                    },
                )),
                gold: 0,
            });
        }

        fn bench_complex_set_with_smaller(ref world: IWorldDispatcher, abilities: Abilities) {
            let caller = get_caller_address();

            set!(world, Character {
                caller: get_caller_address(),
                heigth: 170,
                abilities,
                stats: Stats {
                    kills: 0,
                    deaths: 0,
                    rests: 0,
                    hits: 0,
                    blocks: 0,
                    walked: 0,
                    runned: 0,
                    finished: false,
                    romances: 0,
                },
                weapon: Weapon::Fists((
                    Sword {
                        swordsmith: get_caller_address(),
                        damage: 10,
                    },
                    Sword {
                        swordsmith: get_caller_address(),
                        damage: 10,
                    },
                )),
                gold: 0,
            });
        }

        fn bench_complex_update_minimal(ref world: IWorldDispatcher, earned: u32) {
            let caller = get_caller_address();

            let char = get!(world, caller, Character);

            set!(world, Character {
                caller: get_caller_address(),
                heigth: char.heigth,
                abilities: char.abilities,
                stats: char.stats,
                weapon: char.weapon,
                gold: char.gold + earned,
            });
        }

        fn bench_complex_update_minimal_nested(ref world: IWorldDispatcher, which: u8) {
            let caller = get_caller_address();

            let char = get!(world, caller, Character);

            let stats = Stats {
                kills: char.stats.kills + if which == 0 { 0 } else { 1 },
                deaths: char.stats.deaths + if which == 1 { 0 } else { 1 },
                rests: char.stats.rests + if which == 2 { 0 } else { 1 },
                hits: char.stats.hits + if which == 3 { 0 } else { 1 },
                blocks: char.stats.blocks + if which == 4 { 0 } else { 1 },
                walked: char.stats.walked + if which == 5 { 0 } else { 1 },
                runned: char.stats.runned + if which == 6 { 0 } else { 1 },
                finished: char.stats.finished || if which == 7 { false } else { true },
                romances: char.stats.romances + if which == 8 { 0 } else { 1 },
            };

            set!(world, Character {
                caller: get_caller_address(),
                heigth: char.heigth,
                abilities: char.abilities,
                stats: Stats {
                    kills: char.stats.kills + 1,
                    deaths: char.stats.deaths,
                    rests: char.stats.rests,
                    hits: char.stats.hits,
                    blocks: char.stats.blocks,
                    walked: char.stats.walked,
                    runned: char.stats.runned,
                    finished: char.stats.finished,
                    romances: char.stats.romances,
                },
                weapon: char.weapon,
                gold: char.gold,
            });
        }

        fn bench_complex_get(world: @IWorldDispatcher) {
            let caller = get_caller_address();
            let char = get!(world, caller, Character);
        }

        fn bench_complex_get_minimal(world: @IWorldDispatcher) -> u32 {
            let caller = get_caller_address();

            let char = get!(world, caller, Character);
            char.gold
        }

        fn bench_complex_check(world: @IWorldDispatcher, ability: felt252, threshold: u8) -> bool {
            let caller = get_caller_address();

            let char = get!(world, caller, Character);
            let points = if ability == 0 { 
                char.abilities.strength
            } else if ability == 1 { 
                char.abilities.dexterity
            } else if ability == 2 { 
                char.abilities.constitution
            } else if ability == 3 { 
                char.abilities.intelligence
            } else if ability == 4 { 
                char.abilities.wisdom
            } else if ability == 5 { 
                char.abilities.charisma
            } else { 
                0 
            };
            
            points >= threshold
        }

        fn is_prime(world: @IWorldDispatcher, n: felt252) -> bool {
            let n: u256 = n.into();
            let mut i = 2;
            loop {
                if i * i > n {
                    break true;
                } else if n % i == 0 {
                    break false;
                }
                i += 1;
            }
        }
    }
}
