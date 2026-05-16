pub fn evaluate(expr: &str) -> Result<f64, String> {
    meval::eval_str(expr).map_err(|e| e.to_string())
}
