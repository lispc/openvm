use std::sync::Arc;

use afs_chips::{
    common::page::Page, multitier_page_rw_checker::page_controller::MyLessThanTupleParams,
    range_gate::RangeCheckerGateChip,
};
use afs_stark_backend::{
    keygen::{types::MultiStarkPartialProvingKey, MultiStarkKeygenBuilder},
    prover::{trace::TraceCommitmentBuilder, MultiTraceStarkProver},
    verifier::VerificationError,
};
use afs_test_utils::config;
use afs_test_utils::config::baby_bear_poseidon2::{
    BabyBearPoseidon2Config, BabyBearPoseidon2Engine,
};
use afs_test_utils::utils::create_seeded_rng;
use p3_baby_bear::BabyBear;
use p3_field::AbstractField;
use rand::Rng;

use super::page_controller::{PageController, SplitRemainingProverData};

fn load_page_test(
    engine: &BabyBearPoseidon2Engine,
    remaining: &Page,
    input_page: &Page,
    output_pages: &[Page],
    falsify_is_full_idx: Option<usize>,
    page_controller: &mut PageController<BabyBearPoseidon2Config>,
    trace_builder: &mut TraceCommitmentBuilder<BabyBearPoseidon2Config>,
    partial_pk: &MultiStarkPartialProvingKey<BabyBearPoseidon2Config>,
) -> Result<(), VerificationError> {
    let pdata = page_controller.load_pages(
        remaining,
        input_page,
        output_pages,
        SplitRemainingProverData {
            remaining_pdata: None,
            input_page_pdata: None,
            output_page_pdata: vec![None; output_pages.len()],
        },
        &mut trace_builder.committer,
    );

    let (proof, mut pis) = page_controller.prove(engine, partial_pk, trace_builder, pdata);
    if let Some(idx) = falsify_is_full_idx {
        pis[2 + idx][0] = BabyBear::one();
    }
    for i in 0..output_pages.len() {
        println!("PIS ARE: {:?}", pis[i + 2]);
    }
    page_controller.verify(engine, partial_pk.partial_vk(), proof, pis)
}

#[test]
fn split_remaining_test() {
    let mut rng = create_seeded_rng();

    let page_bus_index = 0;
    let lt_bus_index = 1;

    const MAX_VAL: u32 = 0x78000001 / 2; // The prime used by BabyBear / 2

    let log_page_height = 4;
    let log_num_ops = 3;

    let page_width = 6;

    let idx_len = rng.gen::<usize>() % ((page_width - 1) - 1) + 1;
    let data_len = (page_width - 1) - idx_len;
    let idx_limb_bits = 10;
    let idx_decomp = 4;
    let max_idx = 1 << idx_limb_bits;
    let k = 5;

    let page_height = 1 << log_page_height;

    // Generating a random page with distinct indices
    let input_page = Page::random(
        &mut rng,
        idx_len,
        data_len,
        max_idx,
        MAX_VAL,
        page_height,
        10,
    );
    let remaining = Page::random(
        &mut rng,
        idx_len,
        data_len,
        max_idx,
        MAX_VAL,
        4 * page_height,
        40,
    );
    let range_checker = RangeCheckerGateChip::new(lt_bus_index, 1 << idx_decomp);
    let mut page_controller: PageController<BabyBearPoseidon2Config> = PageController::new(
        page_bus_index,
        idx_len,
        data_len,
        MyLessThanTupleParams {
            limb_bits: idx_limb_bits,
            decomp: idx_decomp,
        },
        Arc::new(range_checker),
        k,
    );
    let engine = config::baby_bear_poseidon2::default_engine(
        idx_decomp.max(log_page_height.max(3 + log_num_ops)),
    );
    let mut keygen_builder = MultiStarkKeygenBuilder::new(&engine.config);

    page_controller.set_up_keygen_builder(&mut keygen_builder);

    let partial_pk = keygen_builder.generate_partial_pk();

    let prover = MultiTraceStarkProver::new(&engine.config);
    let mut trace_builder = TraceCommitmentBuilder::new(prover.pcs());

    let output_pages = page_controller.generate_output_pages(&remaining, &input_page);

    load_page_test(
        &engine,
        &remaining,
        &input_page,
        &output_pages,
        None,
        &mut page_controller,
        &mut trace_builder,
        &partial_pk,
    )
    .expect("Verification failed");

    let result = load_page_test(
        &engine,
        &remaining,
        &input_page,
        &output_pages,
        Some(3),
        &mut page_controller,
        &mut trace_builder,
        &partial_pk,
    );

    assert!(
        result.is_err(),
        "Expected to fail when is_full is wrongly marked"
    );
}
