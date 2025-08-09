pub fn softmax(x: &[f64]) -> Vec<f64> {
    let max = x.iter().cloned().fold(f64::MIN, f64::max);
    let mut exps = x.iter().map(|xi| (*xi - max).exp()).collect::<Vec<f64>>();
    let s = exps.iter().sum::<f64>();
    exps.iter_mut().for_each(|x| *x /= s);
    exps
}
