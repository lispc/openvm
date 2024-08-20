use std::sync::Arc;

use afs_page::common::page::Page;
use datafusion::arrow::{
    array::{ArrayRef, PrimitiveArray, RecordBatch, UInt16Array, UInt32Array, UInt8Array},
    datatypes::{
        ArrowPrimitiveType, DataType, Field, Schema, ToByteSlice, UInt16Type, UInt32Type, UInt8Type,
    },
};

use crate::{
    utils::data_types::{num_bytes, num_fe},
    BITS_PER_FE, NUM_IDX_COLS,
};

/// Converts a Page with Schema to a RecordBatch. A Page is a collection of rows with an is_alloc value as well as idx and data
/// vectors, all of type u32. A RecordBatch is a collection of columns. Page data needs to be collected into columns with datatypes
/// that match the Schema.
pub fn page_to_record_batch(page: Page, schema: Schema) -> RecordBatch {
    let bytes_per_fe = num_bytes(BITS_PER_FE);

    // Get the size of each data type for each field
    let bytes_per_col: Vec<usize> = schema
        .fields()
        .iter()
        .map(|field| {
            let data_type = (**field).data_type();
            data_type.primitive_width().unwrap()
        })
        .collect();
    let mut idx_cols = vec![vec![]; NUM_IDX_COLS];
    let mut data_cols = vec![vec![]; schema.fields().len() - NUM_IDX_COLS];

    // Fill each column with the appropriate number of elements
    for row in &page.rows {
        if row.is_alloc == 0 {
            continue;
        }
        let mut curr_idx = 0;
        let mut curr_data = 0;
        for (i, &num_bytes) in bytes_per_col.iter().enumerate() {
            let skip_bytes = std::cmp::max(4 - bytes_per_fe, 4 - num_bytes);
            let col_fe = num_fe(num_bytes);
            if i < NUM_IDX_COLS {
                let idx_val = row.idx[curr_idx..curr_idx + col_fe]
                    .iter()
                    .flat_map(|&x| {
                        x.to_be_bytes()
                            .iter()
                            .skip(skip_bytes)
                            .cloned()
                            .collect::<Vec<u8>>()
                    })
                    .collect::<Vec<u8>>();
                idx_cols[i].push(idx_val);
                curr_idx += col_fe;
            } else {
                let data_val = row.data[curr_data..curr_data + col_fe]
                    .iter()
                    .flat_map(|&x| {
                        x.to_be_bytes()
                            .iter()
                            .skip(skip_bytes)
                            .cloned()
                            .collect::<Vec<u8>>()
                    })
                    .collect::<Vec<u8>>();
                data_cols[i - NUM_IDX_COLS].push(data_val);
                curr_data += col_fe;
            }
        }
    }

    let mut array_refs: Vec<ArrayRef> = idx_cols
        .into_iter()
        .enumerate()
        .map(|(i, col)| {
            let field = schema.field(i);
            convert_column_to_array(field.data_type(), col)
        })
        .collect();

    array_refs.extend(data_cols.into_iter().enumerate().map(|(i, col)| {
        let field = schema.field(i + NUM_IDX_COLS);
        convert_column_to_array(field.data_type(), col)
    }));

    RecordBatch::try_new(Arc::new(schema), array_refs).unwrap()
}

pub fn record_batch_to_page(rb: &RecordBatch, height: usize) -> Page {
    let bytes_per_fe = num_bytes(BITS_PER_FE);
    let num_rows = rb.num_rows();
    let columns = rb.columns();

    let idx_len = (0..NUM_IDX_COLS).fold(0, |acc, col| {
        let data_type = columns[col].data_type();
        acc + num_fe(data_type.primitive_width().unwrap())
    });
    let data_len = (NUM_IDX_COLS..columns.len()).fold(0, |acc, col| {
        let data_type = columns[col].data_type();
        acc + num_fe(data_type.primitive_width().unwrap())
    });
    let page_width = 1 + idx_len + data_len;

    // Initialize a vec to hold each row, with an extra column for `is_alloc`
    let mut alloc_rows: Vec<Vec<u32>> = vec![vec![1]; num_rows];
    let unalloc_rows: Vec<Vec<u32>> = vec![vec![0; page_width]; height - num_rows];

    // Iterate over columns and fill the rows
    for column in columns {
        let data_type = column.data_type();
        // let num_bytes = data_type.primitive_width().unwrap();
        let col_data = match data_type {
            DataType::UInt8 => extract_column_data::<UInt8Type>(column),
            DataType::UInt16 => extract_column_data::<UInt16Type>(column),
            DataType::UInt32 => extract_column_data::<UInt32Type>(column),
            _ => panic!("Unsupported data type: {}", data_type),
        };
        alloc_rows.iter_mut().enumerate().for_each(|(i, row)| {
            let data = &col_data[i];
            let fe_vec = data
                .chunks(bytes_per_fe)
                .map(|x| {
                    let mut bytes = vec![0; 4 - x.len()];
                    bytes.extend_from_slice(x);
                    u32::from_be_bytes(bytes.try_into().unwrap())
                })
                .collect::<Vec<u32>>();
            row.extend(fe_vec);
        });
    }
    alloc_rows.extend(unalloc_rows);

    Page::from_2d_vec(&alloc_rows, idx_len, data_len)
}

fn convert_column_to_array(data_type: &DataType, col: Vec<Vec<u8>>) -> ArrayRef {
    match data_type {
        DataType::UInt8 => {
            let array: UInt8Array =
                UInt8Array::from(col.into_iter().flatten().collect::<Vec<u8>>());
            Arc::new(array) as ArrayRef
        }
        DataType::UInt16 => {
            let array: UInt16Array = UInt16Array::from(
                col.into_iter()
                    .map(|x| u16::from_be_bytes(x.try_into().unwrap()))
                    .collect::<Vec<u16>>(),
            );
            Arc::new(array) as ArrayRef
        }
        DataType::UInt32 => {
            let array: UInt32Array = UInt32Array::from(
                col.into_iter()
                    .map(|x| u32::from_be_bytes(x.try_into().unwrap()))
                    .collect::<Vec<u32>>(),
            );
            Arc::new(array) as ArrayRef
        }
        _ => panic!("Unsupported data type"),
    }
}

fn extract_column_data<T>(column: &ArrayRef) -> Vec<Vec<u8>>
where
    T: ArrowPrimitiveType,
    T::Native: Copy,
{
    let array = column.as_any().downcast_ref::<PrimitiveArray<T>>().unwrap();
    let array = array.values().to_vec();
    array
        .iter()
        .map(|x| x.to_byte_slice().to_vec().iter().rev().cloned().collect())
        .collect()
}

#[test]
pub fn test_page_to_record_batch() {
    let page = Page::from_2d_vec(
        &[
            vec![1, 1, 2, 3, 4, 5, 6],
            vec![1, 2, 4, 6, 8, 10, 12],
            vec![1, 3, 6, 9, 12, 15, 18],
            vec![0, 0, 0, 0, 0, 0, 0],
        ],
        2,
        4,
    );
    let schema = Schema::new(vec![
        Field::new("idx", DataType::UInt32, false),
        Field::new("d0", DataType::UInt8, false),
        Field::new("d1", DataType::UInt16, false),
        Field::new("d2", DataType::UInt32, false),
    ]);

    let record_batch = page_to_record_batch(page, schema.clone());
    let record_batch_cmp = RecordBatch::try_new(
        Arc::new(schema),
        vec![
            Arc::new(UInt32Array::from(vec![65538, 131076, 196614])),
            Arc::new(UInt8Array::from(vec![3, 6, 9])),
            Arc::new(UInt16Array::from(vec![4, 8, 12])),
            Arc::new(UInt32Array::from(vec![327686, 655372, 983058])),
        ],
    )
    .unwrap();
    assert_eq!(record_batch, record_batch_cmp);
}

#[test]
pub fn test_record_batch_to_page() {
    let idx_col = UInt32Array::from(vec![1, 2, 3, 4, 5]);
    let d0_col = UInt8Array::from(vec![1, 2, 3, 4, 5]);
    let d1_col = UInt16Array::from(vec![2, 4, 6, 8, 10]);
    let d2_col = UInt32Array::from(vec![3, 6, 9, 12, 15]);

    let schema = Schema::new(vec![
        Field::new("idx", DataType::UInt32, false),
        Field::new("d0", DataType::UInt8, false),
        Field::new("d1", DataType::UInt16, false),
        Field::new("d2", DataType::UInt32, false),
    ]);

    let record_batch = RecordBatch::try_new(
        Arc::new(schema),
        vec![
            Arc::new(idx_col),
            Arc::new(d0_col),
            Arc::new(d1_col),
            Arc::new(d2_col),
        ],
    )
    .unwrap();

    let page = record_batch_to_page(&record_batch, 8);
    let page_cmp = Page::from_2d_vec(
        &[
            vec![1, 0, 1, 1, 2, 0, 3],
            vec![1, 0, 2, 2, 4, 0, 6],
            vec![1, 0, 3, 3, 6, 0, 9],
            vec![1, 0, 4, 4, 8, 0, 12],
            vec![1, 0, 5, 5, 10, 0, 15],
            vec![0, 0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 0, 0],
        ],
        2,
        4,
    );
    assert_eq!(page, page_cmp);
}
