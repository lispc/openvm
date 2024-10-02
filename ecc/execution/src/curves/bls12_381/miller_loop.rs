use halo2curves_axiom::ff::Field;
use itertools::{izip, Itertools};

use crate::{
    common::{EcPoint, FieldExtension},
    curves::bls12_381::{
        evaluate_line, fp12_square, mul_023_by_023, mul_by_012345, mul_by_023, q_signed,
    },
};

#[allow(non_snake_case)]
pub fn miller_double_step<Fp, Fp2>(S: EcPoint<Fp2>) -> (EcPoint<Fp2>, [Fp2; 2])
where
    Fp: Field,
    Fp2: FieldExtension<2, BaseField = Fp>,
{
    let one = Fp2::ONE;
    let two = one + one;
    let three = one + two;

    // Calculate 2S
    let x = S.x;
    let y = S.y;
    // λ = (3x^2) / (2y)
    let two_y_inv = (two * y).invert().unwrap();
    let lambda = (three * x.square()) * two_y_inv;
    // x_2S = λ^2 - 2x
    let x_2S = lambda.square() - two * x;
    // y_2S = λ(x - x_2S) - y
    let y_2S = lambda * (x - x_2S) - y;
    let Q_acc = EcPoint { x: x_2S, y: y_2S };

    // Tangent line
    //   1 + b' (x_P / y_P) w + c' (1 / y_P) w^3
    // where
    //   l_{\Psi(S),\Psi(S)}(P) = 1 - λ (x_P / y_P) w + (λ * x_S - y_S) (1 / y_P) w^3
    // b = - λ
    // c = λ * x_S - y_S
    let b = lambda.neg();
    let c = lambda * x - y;

    (Q_acc, [b, c])
}

#[allow(non_snake_case)]
pub fn miller_add_step<Fp, Fp2>(S: EcPoint<Fp2>, Q: EcPoint<Fp2>) -> (EcPoint<Fp2>, [Fp2; 2])
where
    Fp: Field,
    Fp2: FieldExtension<2, BaseField = Fp>,
{
    let x_s = S.x;
    let y_s = S.y;
    let x_q = Q.x;
    let y_q = Q.y;

    // λ1 = (y_s - y_q) / (x_s - x_q)
    let x_s_minus_x_q_inv = (x_s - x_q).invert().unwrap();
    let lambda = (y_s - y_q) * x_s_minus_x_q_inv;
    let x_s_plus_q = lambda.square() - x_s - x_q;
    let y_s_plus_q = lambda * (x_q - x_s_plus_q) - y_q;

    let res = EcPoint {
        x: x_s_plus_q,
        y: y_s_plus_q,
    };

    // l_{\Psi(S),\Psi(Q)}(P) = 1 - λ_1 (x_P / y_P) w + (λ_1 * x_S - y_S) (1 / y_P) w^3
    let b = -lambda;
    let c = lambda * x_s - y_s;

    (res, [b, c])
}

#[allow(non_snake_case)]
pub fn miller_double_and_add_step<Fp, Fp2>(
    S: EcPoint<Fp2>,
    Q: EcPoint<Fp2>,
) -> (EcPoint<Fp2>, [Fp2; 2], [Fp2; 2])
where
    Fp: Field,
    Fp2: FieldExtension<2, BaseField = Fp>,
{
    let one = Fp2::ONE;
    let two = one + one;

    let x_s = S.x;
    let y_s = S.y;
    let x_q = Q.x;
    let y_q = Q.y;

    println!("x_q, x_s,: {:?}, {:?}", x_q, x_s);
    // λ1 = (y_s - y_q) / (x_s - x_q)
    let lambda1 = (y_s - y_q) * (x_s - x_q).invert().unwrap();
    let x_s_plus_q = lambda1.square() - x_s - x_q;

    // λ2 = -λ1 - 2y_s / (x_{s+q} - x_s)
    let lambda2 = lambda1.neg() - two * y_s * (x_s_plus_q - x_s).invert().unwrap();
    let x_s_plus_q_plus_s = lambda2.square() - x_s - x_s_plus_q;
    let y_s_plus_q_plus_s = lambda2 * (x_s - x_s_plus_q_plus_s) - y_s;

    let res = EcPoint {
        x: x_s_plus_q_plus_s,
        y: y_s_plus_q_plus_s,
    };

    // l_{\Psi(S),\Psi(Q)}(P) = 1 - λ_1 (x_P / y_P) w + (λ_1 * x_S - y_S) (1 / y_P) w^3
    let b_1 = -lambda1;
    let c_1 = lambda1 * x_s - y_s;

    // l_{\Psi(S+Q),\Psi(S)}(P) = 1 - λ_2 (x_P / y_P) w + (λ_2 * x_S - y_S) (1 / y_P) w^3
    let b_2 = -lambda2;
    let c_2 = lambda2 * x_s - y_s;

    (res, [b_1, c_1], [b_2, c_2])
}

#[allow(non_snake_case)]
pub fn multi_miller_loop<Fp, Fp2, Fp12>(
    P: &[EcPoint<Fp>],
    Q: &[EcPoint<Fp2>],
    pseudo_binary_encoding: &[i32],
    xi: Fp2,
) -> Fp12
where
    Fp: Field,
    Fp2: FieldExtension<2, BaseField = Fp>,
    Fp12: FieldExtension<6, BaseField = Fp2>,
{
    multi_miller_loop_embedded_exp::<Fp, Fp2, Fp12>(P, Q, None, pseudo_binary_encoding, xi)
}

#[allow(non_snake_case)]
pub fn multi_miller_loop_embedded_exp<Fp, Fp2, Fp12>(
    P: &[EcPoint<Fp>],
    Q: &[EcPoint<Fp2>],
    c: Option<Fp12>,
    pseudo_binary_encoding: &[i32],
    xi: Fp2,
) -> Fp12
where
    Fp: Field,
    Fp2: FieldExtension<2, BaseField = Fp>,
    Fp12: FieldExtension<6, BaseField = Fp2>,
{
    assert!(P.len() > 0);
    assert_eq!(P.len(), Q.len());

    let y_invs = P.iter().map(|P| P.y.invert().unwrap()).collect::<Vec<Fp>>();
    let x_over_ys = P
        .iter()
        .zip(y_invs.iter())
        .map(|(P, y_inv)| P.x * y_inv)
        .collect::<Vec<Fp>>();
    let c_inv = if let Some(c) = c {
        c.invert().unwrap()
    } else {
        Fp12::ONE
    };

    let mut f = Fp12::ONE;
    let mut Q_acc = Q.to_vec();
    let mut lines = Vec::<[Fp2; 2]>::new();

    // Debug counters
    let mut total_double = 0;
    let mut total_double_add = 0;

    // Special case the first iteration of the miller loop with pseudo_binary_encoding = 1:
    // this means that the first step is a double and add, but we need to separate the two steps since the optimized
    // `miller_double_and_add_step` will fail because Q_acc is equal to Q_signed on the first iteration
    println!("miller first iter special case");
    let (Q_out_double, lines_2S) = Q_acc
        .into_iter()
        .map(miller_double_step::<Fp, Fp2>)
        .unzip::<_, _, Vec<_>, Vec<_>>();
    Q_acc = Q_out_double;

    let lines_iter = izip!(lines_2S.iter(), x_over_ys.iter(), y_invs.iter());
    for (line_2S, x_over_y, y_inv) in lines_iter {
        let line = evaluate_line::<Fp, Fp2>(*line_2S, *x_over_y, *y_inv);
        lines.push(line);
    }

    let (Q_out_add, lines_S_plus_Q) = Q_acc
        .iter()
        .zip(Q.iter())
        .map(|(Q_acc, Q)| miller_add_step::<Fp, Fp2>(Q_acc.clone(), Q.clone()))
        .unzip::<_, _, Vec<_>, Vec<_>>();
    Q_acc = Q_out_add;

    let lines_iter = izip!(lines_S_plus_Q.iter(), x_over_ys.iter(), y_invs.iter());
    for (lines_S_plus_Q, x_over_y, y_inv) in lines_iter {
        let line = evaluate_line::<Fp, Fp2>(*lines_S_plus_Q, *x_over_y, *y_inv);
        lines.push(line);
    }

    // Debug counter
    total_double_add += 1;

    for i in (0..pseudo_binary_encoding.len() - 2).rev() {
        println!(
            "miller i: {} = {}; Q_acc.x: {:?}",
            i, pseudo_binary_encoding[i], Q_acc[0].x
        );

        // Regular Miller loop iteration
        f = fp12_square::<Fp12>(f);

        if pseudo_binary_encoding[i] == 0 {
            // Run miller double step if \sigma_i == 0
            let (Q_out, lines_2S) = Q_acc
                .into_iter()
                .map(miller_double_step::<Fp, Fp2>)
                .unzip::<_, _, Vec<_>, Vec<_>>();
            Q_acc = Q_out;

            let lines_iter = izip!(lines_2S.iter(), x_over_ys.iter(), y_invs.iter());
            for (line_2S, x_over_y, y_inv) in lines_iter {
                let line = evaluate_line::<Fp, Fp2>(*line_2S, *x_over_y, *y_inv);
                lines.push(line);
            }

            // Debug counter
            total_double += 1;

            // // Gnark implementation
            // for line in lines.iter() {
            //     f = mul_by_023::<Fp, Fp2, Fp12>(f, *line);
            // }
        } else {
            // use embedded exponent technique if c is provided
            // f = if let Some(c) = c {
            //     match pseudo_binary_encoding[i] {
            //         1 => fp12_multiply(f, c),
            //         -1 => fp12_multiply(f, c_inv),
            //         _ => panic!("Invalid sigma_i"),
            //     }
            // } else {
            //     f
            // };

            // Run miller double and add if \sigma_i != 0
            let Q_signed = q_signed(Q, pseudo_binary_encoding[i]);
            let (Q_out, lines_S_plus_Q, lines_S_plus_Q_plus_S): (Vec<_>, Vec<_>, Vec<_>) = Q_acc
                .iter()
                .zip(Q_signed.iter())
                .map(|(Q_acc, Q_signed)| {
                    miller_double_and_add_step::<Fp, Fp2>(Q_acc.clone(), Q_signed.clone())
                })
                .multiunzip();
            Q_acc = Q_out;

            let lines_iter = izip!(
                lines_S_plus_Q.iter(),
                lines_S_plus_Q_plus_S.iter(),
                x_over_ys.iter(),
                y_invs.iter()
            );
            // let mut lines0 = Vec::<[Fp2; 2]>::new();
            // let mut lines1 = Vec::<[Fp2; 2]>::new();
            for (line_S_plus_Q, line_S_plus_Q_plus_S, x_over_y, y_inv) in lines_iter {
                let line0 = evaluate_line::<Fp, Fp2>(*line_S_plus_Q, *x_over_y, *y_inv);
                let line1 = evaluate_line::<Fp, Fp2>(*line_S_plus_Q_plus_S, *x_over_y, *y_inv);
                // lines0.push(line0);
                // lines1.push(line1);
                lines.push(line0);
                lines.push(line1);
            }
            // let lines_concat = [lines0, lines1].concat();
            // lines.extend(lines_concat);

            // Debug counter
            total_double_add += 1;

            // // Gnark implementation
            // for chunk in lines.chunks(2) {
            //     if let [line0, line1] = chunk {
            //         let prod = mul_023_by_023(*line0, *line1, xi);
            //         f = mul_by_012345(f, prod);
            //     } else {
            //         panic!("lines.len() % 2 should be 0 at this point");
            //     }
            // }
        };
    }
    println!("miller: total double: {total_double}, total double&add: {total_double_add}");

    if lines.len() % 2 == 1 {
        f = mul_by_023::<Fp, Fp2, Fp12>(f, lines.pop().unwrap());
    }
    for chunk in lines.chunks(2) {
        if let [line0, line1] = chunk {
            let prod = mul_023_by_023(*line0, *line1, xi);
            f = mul_by_012345(f, prod);
        } else {
            panic!("lines.len() % 2 should be 0 at this point");
        }
    }

    // NOTE: match gnark implementation
    // f.conjugate();

    f
}

#[allow(non_snake_case)]
pub fn multi_miller_loop_separate_double_plus_add<Fp, Fp2, Fp12>(
    P: &[EcPoint<Fp>],
    Q: &[EcPoint<Fp2>],
    _pseudo_binary_encoding: &[i32],
    xi: Fp2,
) -> Fp12
where
    Fp: Field,
    Fp2: FieldExtension<2, BaseField = Fp>,
    Fp12: FieldExtension<6, BaseField = Fp2>,
{
    pub const BLS_X: u64 = 0xd201_0000_0001_0000;
    assert!(P.len() > 0);
    assert_eq!(P.len(), Q.len());

    let y_invs = P.iter().map(|P| P.y.invert().unwrap()).collect::<Vec<Fp>>();
    let x_over_ys = P
        .iter()
        .zip(y_invs.iter())
        .map(|(P, y_inv)| P.x * y_inv)
        .collect::<Vec<Fp>>();

    let mut f = Fp12::ONE;
    let mut Q_acc = Q.to_vec();

    let mut total_double = 0;
    let mut total_double_add = 0;

    let mut found_one = false;
    for i in (0..64).rev().map(|b| (((BLS_X >> 1) >> b) & 1) == 1) {
        if !found_one {
            found_one = i;
            continue;
        }
        let i_binary = i as u8;
        println!("miller i: {} = {}; Q_acc.x: {:?}", i, i_binary, Q_acc[0].x);
        let mut lines = Vec::<[Fp2; 2]>::new();

        // Do double step
        let (Q_out, lines_2S) = Q_acc
            .into_iter()
            .map(miller_double_step::<Fp, Fp2>)
            .unzip::<_, _, Vec<_>, Vec<_>>();
        Q_acc = Q_out;

        let lines_iter = izip!(lines_2S.iter(), x_over_ys.iter(), y_invs.iter());
        for (line_2S, x_over_y, y_inv) in lines_iter {
            let line = &evaluate_line::<Fp, Fp2>(*line_2S, *x_over_y, *y_inv);
            lines.push(*line);
        }

        // Gnark implementation
        // for line in lines.iter() {
        //     f = mul_by_023::<Fp, Fp2, Fp12>(f, *line);
        // }
        // lines = Vec::new();

        if i {
            // Do add step
            let (Q_out_add, lines_S_plus_Q) = Q_acc
                .iter()
                .zip(Q.iter())
                .map(|(Q_acc, Q)| miller_add_step::<Fp, Fp2>(Q_acc.clone(), Q.clone()))
                .unzip::<_, _, Vec<_>, Vec<_>>();
            Q_acc = Q_out_add;

            let lines_iter = izip!(
                lines_2S.iter(),
                lines_S_plus_Q.iter(),
                x_over_ys.iter(),
                y_invs.iter()
            );
            let mut lines0 = Vec::<[Fp2; 2]>::new();
            let mut lines1 = Vec::<[Fp2; 2]>::new();
            for (line0, line1, x_over_y, y_inv) in lines_iter {
                let line0 = &evaluate_line::<Fp, Fp2>(*line0, *x_over_y, *y_inv);
                let line1 = &evaluate_line::<Fp, Fp2>(*line1, *x_over_y, *y_inv);
                lines0.push(*line0);
                lines1.push(*line1);
                // lines.push(*line0);
                // lines.push(*line1);
            }
            let lines_concat = [lines0, lines1].concat();
            lines.extend(lines_concat);

            // Debug counter
            total_double_add += 1;

            // // Gnark implementation
            // for chunk in lines.chunks(2) {
            //     if let [line0, line1] = chunk {
            //         let prod = mul_023_by_023(*line0, *line1, xi);
            //         f = mul_by_012345(f, prod);
            //     } else {
            //         panic!("lines.len() % 2 should be 0 at this point");
            //     }
            // }
        } else {
            // Debug counter
            total_double += 1;
        }

        if lines.len() % 2 == 1 {
            f = mul_by_023::<Fp, Fp2, Fp12>(f, lines.pop().unwrap());
        }
        for chunk in lines.chunks(2) {
            if let [line0, line1] = chunk {
                let prod = mul_023_by_023(*line0, *line1, xi);
                f = mul_by_012345(f, prod);
            } else {
                panic!("lines.len() % 2 should be 0 at this point");
            }
        }

        f = fp12_square::<Fp12>(f);
    }
    let mut lines = Vec::<[Fp2; 2]>::new();

    // Do double step
    let (Q_out, lines_2S) = Q_acc
        .into_iter()
        .map(miller_double_step::<Fp, Fp2>)
        .unzip::<_, _, Vec<_>, Vec<_>>();
    Q_acc = Q_out;

    let lines_iter = izip!(lines_2S.iter(), x_over_ys.iter(), y_invs.iter());
    for (line_2S, x_over_y, y_inv) in lines_iter {
        let line = &evaluate_line::<Fp, Fp2>(*line_2S, *x_over_y, *y_inv);
        lines.push(*line);
    }

    // Gnark implementation
    for line in lines.iter() {
        f = mul_by_023::<Fp, Fp2, Fp12>(f, *line);
    }

    // Debug counter
    total_double += 1;

    println!("miller: total double: {total_double}, total double&add: {total_double_add}");

    f.conjugate();

    f
}
