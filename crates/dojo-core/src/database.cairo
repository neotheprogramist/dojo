use array::{ArrayTrait, SpanTrait};
use traits::{Into, TryInto};
use serde::Serde;
use hash::LegacyHash;
use poseidon::poseidon_hash_span;

mod index;
#[cfg(test)]
mod index_test;
mod schema;
mod storage;
#[cfg(test)]
mod storage_test;
mod utils;
#[cfg(test)]
mod utils_test;

use index::WhereCondition;

// Could be replaced with a `KeyValues` with one value, 
// but this allows us to avoid hashing, and is most common.
#[derive(Copy, Drop, Serde)]
struct MemberClause {
    model: felt252,
    member: felt252,
    value: felt252,
}

#[derive(Copy, Drop, Serde)]
struct CompositeClause {
    operator: LogicalOperator,
    clauses: Span<Clause>,
}

#[derive(Copy, Drop, Serde)]
enum LogicalOperator {
    And,
}

#[derive(Copy, Drop, Serde)]
enum Clause {
    Member: MemberClause,
    Composite: CompositeClause,
    All: felt252,
}

fn get(model: felt252, key: felt252, offset: u8, length: usize, layout: Span<u8>) -> Span<felt252> {
    let mut keys = ArrayTrait::new();
    keys.append('dojo_storage');
    keys.append(model);
    keys.append(key);
    storage::get_many(0, keys.span(), offset, length, layout)
}

fn set(model: felt252, key: felt252, offset: u8, value: Span<felt252>, layout: Span<u8>) {
    let mut keys = ArrayTrait::new();
    keys.append('dojo_storage');
    keys.append(model);
    keys.append(key);
    storage::set_many(0, keys.span(), offset, value, layout);
}

/// Creates an entry in the database and adds it to appropriate indexes.
/// # Arguments
/// * `model` - The model to create the entry in.
/// * `id` - id of the created entry, hash of all `key_values`.
/// * `offset` - The offset of the entry.
/// * `key_names` - The members to create an index on.
/// * `key_values` - The members to create an index on (or hash of values if longer than felt252).
/// * `value` - The value of the entry.
/// * `layout` - The layout of the entry.
fn set_with_index(
    model: felt252,
    id: felt252,
    offset: u8,
    key_names: Span<felt252>,
    key_values: Span<felt252>,
    values: Span<felt252>,
    layout: Span<u8>
) {
    set(model, id, offset, values, layout);

    // If entry is alredy in the indexes we don't need to do anything else.
    if index::exists(0, model, id) {
        return ();
    }

    index::create(0, model, id, 0); // create a record in index of all records

    assert(key_names.len() == key_values.len(), 'keys must be same len');
    let mut idx = 0;
    loop {
        if idx == key_names.len() {
            break;
        } 

        let index = poseidon_hash_span(array![model, *key_names.at(idx)].span());
        index::create(0, index, id, *key_values.at(idx)); // create a record for each of the indexes
        idx += 1;
    };
}

/// Remove an entry in the database and all indexes.
/// # Arguments
/// * `model` - The model containg the entry.
/// * `id` - id of the the entry.
/// * `key_names` - The members on which indexes were created.
fn del(model: felt252, key: felt252, key_names: Span<felt252>) {
    index::delete(0, model, key);

    let mut idx = 0;
    loop {
        if idx == key_names.len() {
            break;
        }
        let index = poseidon_hash_span(array![model, *key_names.at(idx)].span());

        index::delete(0, index, key);
        idx += 1;
    };
}

// Query all entities that meet a criteria. If no index is defined,
// Returns a tuple of spans, first contains the entity IDs,
// second the deserialized entities themselves.
fn scan(where: Clause, values_length: usize, values_layout: Span<u8>) -> Span<Span<felt252>> {
    match where {
        Clause::Member(clause) => {
            let i = poseidon_hash_span(array![clause.model, clause.member].span());
            let keys = index::get(0, i, clause.value);
            get_by_keys(clause.model, keys, values_length, values_layout)
        },
        Clause::Composite(clause) => {
            assert(false, 'unimplemented');
            array![array![].span()].span()
        },
        Clause::All(model) => {
            let keys = index::get(0, model, 0);
            get_by_keys(model, keys, values_length, values_layout)
        }
    }
}

/// Analogous to `scan`, but returns only the keys of the entities.
fn scan_keys(where: Clause) -> Span<felt252> {
    match where {
        Clause::Member(clause) => {
            let i = poseidon_hash_span(array![clause.model, clause.member].span());
            index::get(0, i, clause.value)
        },
        Clause::Composite(clause) => {
            assert(false, 'unimplemented');
            array![].span()
        },
        Clause::All(model) => {
            index::get(0, model, 0)
        }
    }
}

/// Returns entries on the given keys.
/// # Arguments
/// * `model` - The model to get the entries from.
/// * `keys` - The keys of the entries to get.
/// * `length` - The length of the entries.
fn get_by_keys(
    model: felt252, mut keys: Span<felt252>, length: u32, layout: Span<u8>
) -> Span<Span<felt252>> {
    let mut entities: Array<Span<felt252>> = ArrayTrait::new();

    loop {
        match keys.pop_front() {
            Option::Some(key) => {
                let keys = array!['dojo_storage', model, *key];
                let value: Span<felt252> = storage::get_many(0, keys.span(), 0_u8, length, layout);
                entities.append(value);
            },
            Option::None(_) => {
                break entities.span();
            }
        };
    }
}
