// Copyright 2022 Greptime Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod write_batch_util;

use std::sync::Arc;

use datatypes::prelude::ScalarVector;
use datatypes::type_id::LogicalTypeId;
use datatypes::vectors::{
    BooleanVector, Float64Vector, StringVector, TimestampMillisecondVector, UInt64Vector,
};
use rand::Rng;
use storage::proto;
use storage::write_batch::{PutData, WriteBatch};
use store_api::storage::{consts, PutOperation, WriteRequest};

pub fn new_test_batch() -> WriteBatch {
    write_batch_util::new_write_batch(
        &[
            ("k1", LogicalTypeId::UInt64, false),
            (consts::VERSION_COLUMN_NAME, LogicalTypeId::UInt64, false),
            ("ts", LogicalTypeId::TimestampMillisecond, false),
            ("v1", LogicalTypeId::Boolean, true),
            ("4", LogicalTypeId::Float64, false),
            ("5", LogicalTypeId::Float64, false),
            ("6", LogicalTypeId::Float64, false),
            ("7", LogicalTypeId::Float64, false),
            ("8", LogicalTypeId::Float64, false),
            ("9", LogicalTypeId::Float64, false),
            ("10", LogicalTypeId::String, false),
        ],
        Some(2),
    )
}

pub fn gen_new_batch_and_types(putdate_nums: usize) -> (WriteBatch, Vec<i32>) {
    let mut batch = new_test_batch();
    let mut rng = rand::thread_rng();
    for _ in 0..putdate_nums {
        let mut intvs = [0u64; 10];
        let mut boolvs = [true; 10];
        let mut tsvs = [0i64; 10];
        let mut fvs = [0.0_f64; 10];
        let svs = [
            "value1_string",
            "value2_string",
            "value3_string",
            "value4_string",
            "value5_string",
            "value6_string",
            "value7_string",
            "value8_string",
            "value9_string",
            "value10_string",
        ];
        rng.fill(&mut intvs[..]);
        rng.fill(&mut boolvs[..]);
        rng.fill(&mut tsvs[..]);
        rng.fill(&mut fvs[..]);
        let intv = Arc::new(UInt64Vector::from_slice(&intvs));
        let boolv = Arc::new(BooleanVector::from(boolvs.to_vec()));
        let tsv = Arc::new(TimestampMillisecondVector::from_values(tsvs));
        let fvs = Arc::new(Float64Vector::from_slice(&fvs));
        let svs = Arc::new(StringVector::from_slice(&svs));
        let mut put_data = PutData::default();
        put_data.add_key_column("k1", intv.clone()).unwrap();
        put_data.add_version_column(intv).unwrap();
        put_data.add_value_column("v1", boolv).unwrap();
        put_data.add_key_column("ts", tsv.clone()).unwrap();
        put_data.add_key_column("4", fvs.clone()).unwrap();
        put_data.add_key_column("5", fvs.clone()).unwrap();
        put_data.add_key_column("6", fvs.clone()).unwrap();
        put_data.add_key_column("7", fvs.clone()).unwrap();
        put_data.add_key_column("8", fvs.clone()).unwrap();
        put_data.add_key_column("9", fvs.clone()).unwrap();
        put_data.add_key_column("10", svs.clone()).unwrap();
        batch.put(put_data).unwrap();
    }
    let types = proto::wal::gen_mutation_types(&batch);
    (batch, types)
}
