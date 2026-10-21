use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PythonConstraint {
    pub r#type: String,
    pub expr: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PythonProblem {
    pub objective: String,
    pub x0: Vec<f64>,
    pub bounds: Option<Vec<(Option<f64>, Option<f64>)>>,
    pub constraints: Vec<PythonConstraint>,
    pub tol: f64,
    pub maxiter: usize,
}

#[derive(Clone)]
pub enum TestConstraint {
    Eq {
        expr: String,
        fun: fn(&[f64]) -> f64,
    },
    Ineq {
        expr: String,
        fun: fn(&[f64]) -> f64,
    },
}

#[derive(Clone)]
pub struct TestCase {
    pub name: String,
    pub objective_expr: String,
    pub objective_fn: fn(&[f64]) -> f64,
    pub x0: Vec<f64>,
    pub bounds: Option<Vec<(f64, f64)>>,
    pub constraints: Vec<TestConstraint>,
    pub tol: f64,
    pub maxiter: usize,
    /// Maximum allowed deviation when comparing with SciPy (None to use global default)
    pub comparison_tol: Option<f64>,
}

impl TestCase {
    pub fn to_python_problem(&self) -> PythonProblem {
        let constraints = self
            .constraints
            .iter()
            .map(|c| match c {
                TestConstraint::Eq { expr, .. } => PythonConstraint {
                    r#type: "eq".to_string(),
                    expr: expr.clone(),
                },
                TestConstraint::Ineq { expr, .. } => PythonConstraint {
                    r#type: "ineq".to_string(),
                    expr: expr.clone(),
                },
            })
            .collect();

        let bounds = self
            .bounds
            .as_ref()
            .map(|b| b.iter().map(|(l, u)| (Some(*l), Some(*u))).collect());

        PythonProblem {
            objective: self.objective_expr.clone(),
            x0: self.x0.clone(),
            bounds,
            constraints,
            tol: self.tol,
            maxiter: self.maxiter,
        }
    }
}

pub fn create_test_cases() -> Vec<TestCase> {
    vec![
        // 1. Unconstrained optimization: Rosenbrock function (HS1)
        TestCase {
            name: "HS1 (Rosenbrock unconstrained)".to_string(),
            objective_expr: "100.0 * (x[1] - x[0]**2)**2 + (1.0 - x[0])**2".to_string(),
            objective_fn: |x| 100.0 * (x[1] - x[0].powi(2)).powi(2) + (1.0 - x[0]).powi(2),
            x0: vec![-1.2, 1.0],
            bounds: None,
            constraints: vec![],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS2
        TestCase {
            name: "HS2".to_string(),
            objective_expr: "100.0 * (x[1] - x[0]**2)**2 + (1.0 - x[0])**2".to_string(),
            objective_fn: |x| 100.0 * (x[1] - x[0].powi(2)).powi(2) + (1.0 - x[0]).powi(2),
            x0: vec![-1.2, 1.0],
            bounds: Some(vec![(-1e10, 1e10), (0.0, 1e10)]),
            constraints: vec![],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS3
        TestCase {
            name: "HS3".to_string(),
            objective_expr: "x[1] + 1e-5 * (x[1] - x[0])**2".to_string(),
            objective_fn: |x| x[1] + 1e-5 * (x[1] - x[0]).powi(2),
            x0: vec![10.0, 1.0],
            bounds: Some(vec![(-1e10, 1e10), (0.0, 1e10)]),
            constraints: vec![],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS4
        TestCase {
            name: "HS4".to_string(),
            objective_expr: "1.0/3.0 * (x[0] + 1.0)**3 + x[1]".to_string(),
            objective_fn: |x| 1.0/3.0 * (x[0] + 1.0).powi(3) + x[1],
            x0: vec![1.125, 0.125],
            bounds: Some(vec![(1.0, 1e10), (0.0, 1e10)]),
            constraints: vec![],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS5
        TestCase {
            name: "HS5".to_string(),
            objective_expr: "sin(x[0] + x[1]) + (x[0] - x[1])**2 - 1.5 * x[0] + 2.5 * x[1] + 1.0".to_string(),
            objective_fn: |x| (x[0] + x[1]).sin() + (x[0] - x[1]).powi(2) - 1.5 * x[0] + 2.5 * x[1] + 1.0,
            x0: vec![0.0, 0.0],
            bounds: Some(vec![(-1.5, 4.0), (-3.0, 3.0)]),
            constraints: vec![],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS6
        TestCase {
            name: "HS6".to_string(),
            objective_expr: "(1.0 - x[0])**2".to_string(),
            objective_fn: |x| (1.0 - x[0]).powi(2),
            x0: vec![-1.2, 1.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "10.0 * (x[1] - x[0]**2)".to_string(),
                    fun: |x| 10.0 * (x[1] - x[0].powi(2)),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS7
        TestCase {
            name: "HS7".to_string(),
            objective_expr: "log(1.0 + x[0]**2) - x[1]".to_string(),
            objective_fn: |x| (1.0 + x[0].powi(2)).ln() - x[1],
            x0: vec![2.0, 2.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "(1.0 + x[0]**2)**2 + x[1]**2 - 4.0".to_string(),
                    fun: |x| (1.0 + x[0].powi(2)).powi(2) + x[1].powi(2) - 4.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS8
        TestCase {
            name: "HS8".to_string(),
            objective_expr: "-1.0".to_string(),
            objective_fn: |_x| -1.0,
            x0: vec![2.0, 1.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0]**2 + x[1]**2 - 25.0".to_string(),
                    fun: |x| x[0].powi(2) + x[1].powi(2) - 25.0,
                },
                TestConstraint::Eq {
                    expr: "x[0] * x[1] - 9.0".to_string(),
                    fun: |x| x[0] * x[1] - 9.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS9
        TestCase {
            name: "HS9".to_string(),
            objective_expr: "sin(pi * x[0] / 12.0) * cos(pi * x[1] / 16.0)".to_string(),
            objective_fn: |x| (std::f64::consts::PI * x[0] / 12.0).sin() * (std::f64::consts::PI * x[1] / 16.0).cos(),
            x0: vec![0.0, 0.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + x[1] - 12.0".to_string(),
                    fun: |x| x[0] + x[1] - 12.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS10
        TestCase {
            name: "HS10".to_string(),
            objective_expr: "x[0] - x[1]".to_string(),
            objective_fn: |x| x[0] - x[1],
            x0: vec![-10.0, 10.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "1.0 - (3.0 * x[0]**2 - 2.0 * x[0] * x[1] + x[1]**2)".to_string(),
                    fun: |x| 1.0 - (3.0 * x[0].powi(2) - 2.0 * x[0] * x[1] + x[1].powi(2)),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS11
        TestCase {
            name: "HS11".to_string(),
            objective_expr: "(x[0] - 5.0)**2 + x[1]**2 - 25.0".to_string(),
            objective_fn: |x| (x[0] - 5.0).powi(2) + x[1].powi(2) - 25.0,
            x0: vec![4.9, 0.1],
            bounds: None,
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[1] - x[0]**2".to_string(),
                    fun: |x| x[1] - x[0].powi(2),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS12
        TestCase {
            name: "HS12".to_string(),
            objective_expr: "0.5 * x[0]**2 + x[1]**2 - x[0] * x[1] - 7.0 * x[0] - 7.0 * x[1]".to_string(),
            objective_fn: |x| 0.5 * x[0].powi(2) + x[1].powi(2) - x[0] * x[1] - 7.0 * x[0] - 7.0 * x[1],
            x0: vec![0.0, 0.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "25.0 - 4.0 * x[0]**2 - x[1]**2".to_string(),
                    fun: |x| 25.0 - 4.0 * x[0].powi(2) - x[1].powi(2),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS13
        TestCase {
            name: "HS13".to_string(),
            objective_expr: "(x[0] - 2.0)**2 + x[1]**2".to_string(),
            objective_fn: |x| (x[0] - 2.0).powi(2) + x[1].powi(2),
            x0: vec![-2.0, -2.0],
            bounds: Some(vec![(0.0, 1e10), (0.0, 1e10)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "(1.0 - x[0])**3 - x[1]".to_string(),
                    fun: |x| (1.0 - x[0]).powi(3) - x[1],
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: Some(2e-3),
        },
        // HS14
        TestCase {
            name: "HS14".to_string(),
            objective_expr: "(x[0] - 2.0)**2 + (x[1] - 1.0)**2".to_string(),
            objective_fn: |x| (x[0] - 2.0).powi(2) + (x[1] - 1.0).powi(2),
            x0: vec![2.0, 2.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] - 2.0 * x[1] + 1.0".to_string(),
                    fun: |x| x[0] - 2.0 * x[1] + 1.0,
                },
                TestConstraint::Ineq {
                    expr: "1.0 - 0.25 * x[0]**2 - x[1]**2".to_string(),
                    fun: |x| 1.0 - 0.25 * x[0].powi(2) - x[1].powi(2),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS15
        TestCase {
            name: "HS15".to_string(),
            objective_expr: "100.0 * (x[1] - x[0]**2)**2 + (1.0 - x[0])**2".to_string(),
            objective_fn: |x| 100.0 * (x[1] - x[0].powi(2)).powi(2) + (1.0 - x[0]).powi(2),
            x0: vec![-2.0, 1.0],
            bounds: Some(vec![(-1e10, 0.5), (-1e10, 1e10)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[0] * x[1] - 1.0".to_string(),
                    fun: |x| x[0] * x[1] - 1.0,
                },
                TestConstraint::Ineq {
                    expr: "x[0] + x[1]**2".to_string(),
                    fun: |x| x[0] + x[1].powi(2),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS16
        TestCase {
            name: "HS16".to_string(),
            objective_expr: "100.0 * (x[1] - x[0]**2)**2 + (1.0 - x[0])**2".to_string(),
            objective_fn: |x| 100.0 * (x[1] - x[0].powi(2)).powi(2) + (1.0 - x[0]).powi(2),
            x0: vec![-2.0, 1.0],
            bounds: Some(vec![(-0.5, 0.5), (-1e10, 1.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[0] + x[1]**2".to_string(),
                    fun: |x| x[0] + x[1].powi(2),
                },
                TestConstraint::Ineq {
                    expr: "x[0]**2 + x[1]".to_string(),
                    fun: |x| x[0].powi(2) + x[1],
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS17
        TestCase {
            name: "HS17".to_string(),
            objective_expr: "100.0 * (x[1] - x[0]**2)**2 + (1.0 - x[0])**2".to_string(),
            objective_fn: |x| 100.0 * (x[1] - x[0].powi(2)).powi(2) + (1.0 - x[0]).powi(2),
            x0: vec![-2.0, 1.0],
            bounds: Some(vec![(-0.5, 0.5), (0.0, 1e10)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[0] - x[1]**2".to_string(),
                    fun: |x| x[0] - x[1].powi(2),
                },
                TestConstraint::Ineq {
                    expr: "x[0]**2 - x[1]".to_string(),
                    fun: |x| x[0].powi(2) - x[1],
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS18
        TestCase {
            name: "HS18".to_string(),
            objective_expr: "0.01 * x[0]**2 + x[1]**2 - 100.0".to_string(),
            objective_fn: |x| 0.01 * x[0].powi(2) + x[1].powi(2) - 100.0,
            x0: vec![2.0, 2.0],
            bounds: Some(vec![(2.0, 50.0), (0.0, 50.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[0] * x[1] - 10.0".to_string(),
                    fun: |x| x[0] * x[1] - 10.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS19
        TestCase {
            name: "HS19".to_string(),
            objective_expr: "(x[0] - 10.0)**3 + (x[1] - 20.0)**3".to_string(),
            objective_fn: |x| (x[0] - 10.0).powi(3) + (x[1] - 20.0).powi(3),
            x0: vec![20.1, 5.84],
            bounds: Some(vec![(13.0, 100.0), (0.0, 100.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "(x[0] - 5.0)**2 + (x[1] - 5.0)**2 - 100.0".to_string(),
                    fun: |x| (x[0] - 5.0).powi(2) + (x[1] - 5.0).powi(2) - 100.0,
                },
                TestConstraint::Ineq {
                    expr: "82.81 - (x[0] - 6.0)**2 - (x[1] - 5.0)**2".to_string(),
                    fun: |x| 82.81 - (x[0] - 6.0).powi(2) - (x[1] - 5.0).powi(2),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS20
        TestCase {
            name: "HS20".to_string(),
            objective_expr: "100.0 * (x[1] - x[0]**2)**2 + (1.0 - x[0])**2".to_string(),
            objective_fn: |x| 100.0 * (x[1] - x[0].powi(2)).powi(2) + (1.0 - x[0]).powi(2),
            x0: vec![-2.0, 1.0],
            bounds: Some(vec![(-0.5, 0.5), (-1e10, 1e10)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[0] + x[1]**2".to_string(),
                    fun: |x| x[0] + x[1].powi(2),
                },
                TestConstraint::Ineq {
                    expr: "x[0]**2 + x[1]".to_string(),
                    fun: |x| x[0].powi(2) + x[1],
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS21
        TestCase {
            name: "HS21".to_string(),
            objective_expr: "0.01 * x[0]**2 + x[1]**2 - 100.0".to_string(),
            objective_fn: |x| 0.01 * x[0].powi(2) + x[1].powi(2) - 100.0,
            x0: vec![2.0, 2.0],
            bounds: Some(vec![(2.0, 50.0), (-50.0, 50.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[0] * x[1] - 10.0".to_string(),
                    fun: |x| x[0] * x[1] - 10.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS22
        TestCase {
            name: "HS22".to_string(),
            objective_expr: "(x[0] - 2.0)**2 + (x[1] - 1.0)**2".to_string(),
            objective_fn: |x| (x[0] - 2.0).powi(2) + (x[1] - 1.0).powi(2),
            x0: vec![2.0, 2.0],
            bounds: Some(vec![(0.0, 1e10), (0.0, 1e10)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "2.0 - x[0] - x[1]".to_string(),
                    fun: |x| 2.0 - x[0] - x[1],
                },
                TestConstraint::Ineq {
                    expr: "x[0]**2 - x[1]".to_string(),
                    fun: |x| x[0].powi(2) - x[1],
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS23
        TestCase {
            name: "HS23".to_string(),
            objective_expr: "x[0]**2 + x[1]**2".to_string(),
            objective_fn: |x| x[0].powi(2) + x[1].powi(2),
            x0: vec![3.0, 1.0],
            bounds: Some(vec![(-50.0, 50.0), (-50.0, 50.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[0] + x[1] - 1.0".to_string(),
                    fun: |x| x[0] + x[1] - 1.0,
                },
                TestConstraint::Ineq {
                    expr: "x[0]**2 + x[1]**2 - 1.0".to_string(),
                    fun: |x| x[0].powi(2) + x[1].powi(2) - 1.0,
                },
                TestConstraint::Ineq {
                    expr: "9.0 * x[0]**2 + x[1]**2 - 9.0".to_string(),
                    fun: |x| 9.0 * x[0].powi(2) + x[1].powi(2) - 9.0,
                },
                TestConstraint::Ineq {
                    expr: "x[0]**2 - x[1]".to_string(),
                    fun: |x| x[0].powi(2) - x[1],
                },
                TestConstraint::Ineq {
                    expr: "x[1]**2 - x[0]".to_string(),
                    fun: |x| x[1].powi(2) - x[0],
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS24
        TestCase {
            name: "HS24".to_string(),
            objective_expr: "((x[0] - 3.0)**2) / 9.0 + ((x[1] - 2.0)**2) / 4.0".to_string(),
            objective_fn: |x| (x[0] - 3.0).powi(2) / 9.0 + (x[1] - 2.0).powi(2) / 4.0,
            x0: vec![1.0, 1.0],
            bounds: Some(vec![(0.0, 1e10), (0.0, 1e10)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[0] / 1.7320508075688772 - x[1]".to_string(),
                    fun: |x| x[0] / 3.0f64.sqrt() - x[1],
                },
                TestConstraint::Ineq {
                    expr: "x[0] + 1.7320508075688772 * x[1]".to_string(),
                    fun: |x| x[0] + 3.0f64.sqrt() * x[1],
                },
                TestConstraint::Ineq {
                    expr: "6.0 - x[0] - 1.7320508075688772 * x[1]".to_string(),
                    fun: |x| 6.0 - x[0] - 3.0f64.sqrt() * x[1],
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS25
        TestCase {
            name: "HS25".to_string(),
            objective_expr: "sum((exp(-(abs(x[0]-0.01*i))**x[1]/x[2]) - 0.01*i)**2 for i in range(1, 100))".to_string(),
            objective_fn: |x| {
                (1..100).map(|i| {
                    let ui = 0.01 * (i as f64);
                    ((-(x[0] - ui).abs().powf(x[1]) / x[2]).exp() - ui).powi(2)
                }).sum()
            },
            x0: vec![100.0, 12.5, 3.0],
            bounds: Some(vec![(0.1, 100.0), (1.0, 100.0), (0.0, 100.0)]),
            constraints: vec![],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS26
        TestCase {
            name: "HS26".to_string(),
            objective_expr: "(x[0] - x[1])**2 + (x[1] - x[2])**4".to_string(),
            objective_fn: |x| (x[0] - x[1]).powi(2) + (x[1] - x[2]).powi(4),
            x0: vec![-2.6, 2.0, 2.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "(1.0 + x[1]**2) * x[0] + x[2]**4 - 3.0".to_string(),
                    fun: |x| (1.0 + x[1].powi(2)) * x[0] + x[2].powi(4) - 3.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS27
        TestCase {
            name: "HS27".to_string(),
            objective_expr: "0.5 * x[0]**2 + x[1]**2 + 0.5 * x[2]**2 - x[0] * x[1] - x[2]".to_string(),
            objective_fn: |x| 0.5 * x[0].powi(2) + x[1].powi(2) + 0.5 * x[2].powi(2) - x[0] * x[1] - x[2],
            x0: vec![2.0, 2.0, 2.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "1.0 - (x[0] + 2.0 * x[1] + x[2])".to_string(),
                    fun: |x| 1.0 - (x[0] + 2.0 * x[1] + x[2]),
                },
                TestConstraint::Eq {
                    expr: "x[0]**2 + x[1]**2 + x[2]**2 - 1.0".to_string(),
                    fun: |x| x[0].powi(2) + x[1].powi(2) + x[2].powi(2) - 1.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS28
        TestCase {
            name: "HS28".to_string(),
            objective_expr: "(x[0] + x[1])**2 + (x[1] + x[2])**2".to_string(),
            objective_fn: |x| (x[0] + x[1]).powi(2) + (x[1] + x[2]).powi(2),
            x0: vec![-4.0, 1.0, 1.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + 2.0 * x[1] + 3.0 * x[2] - 1.0".to_string(),
                    fun: |x| x[0] + 2.0 * x[1] + 3.0 * x[2] - 1.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS29
        TestCase {
            name: "HS29".to_string(),
            objective_expr: "-x[0] * x[1] * x[2]".to_string(),
            objective_fn: |x| -x[0] * x[1] * x[2],
            x0: vec![1.0, 1.0, 1.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "48.0 - (x[0]**2 + 2.0 * x[1]**2 + 4.0 * x[2]**2)".to_string(),
                    fun: |x| 48.0 - (x[0].powi(2) + 2.0 * x[1].powi(2) + 4.0 * x[2].powi(2)),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS30
        TestCase {
            name: "HS30".to_string(),
            objective_expr: "x[0]**2 + x[1]**2 + x[2]**2".to_string(),
            objective_fn: |x| x[0].powi(2) + x[1].powi(2) + x[2].powi(2),
            x0: vec![1.0, 1.0, 1.0],
            bounds: Some(vec![(1.0, 10.0), (1.0, 10.0), (-10.0, 10.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[2]**2 - x[0]**2 - x[1]**2".to_string(),
                    fun: |x| x[2].powi(2) - x[0].powi(2) - x[1].powi(2),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS31
        TestCase {
            name: "HS31".to_string(),
            objective_expr: "x[0]**2 + x[1]**2 + x[2]**2".to_string(),
            objective_fn: |x| x[0].powi(2) + x[1].powi(2) + x[2].powi(2),
            x0: vec![1.0, 1.0, 1.0],
            bounds: Some(vec![(1.0, 10.0), (1.0, 10.0), (1.0, 10.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[0] * x[1] - 1.0".to_string(),
                    fun: |x| x[0] * x[1] - 1.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS32
        TestCase {
            name: "HS32".to_string(),
            objective_expr: "(x[0] + 3.0 * x[1] + x[2])**2 + 4.0 * (x[0] - x[1])**2".to_string(),
            objective_fn: |x| (x[0] + 3.0 * x[1] + x[2]).powi(2) + 4.0 * (x[0] - x[1]).powi(2),
            x0: vec![0.1, 0.7, 0.2],
            bounds: Some(vec![(0.0, 1e10), (0.0, 1e10), (0.0, 1e10)]),
            constraints: vec![
                TestConstraint::Eq {
                    expr: "1.0 - x[0] - x[1] - x[2]".to_string(),
                    fun: |x| 1.0 - x[0] - x[1] - x[2],
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS33
        TestCase {
            name: "HS33".to_string(),
            objective_expr: "(x[0] - 1.0) * (x[0] - 2.0) * (x[0] - 3.0) + x[2]".to_string(),
            objective_fn: |x| (x[0] - 1.0) * (x[0] - 2.0) * (x[0] - 3.0) + x[2],
            x0: vec![0.0, 0.0, 3.0],
            bounds: Some(vec![(0.0, 1e10), (0.0, 1e10), (0.0, 5.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[2]**2 - x[1]**2 - x[0]**2".to_string(),
                    fun: |x| x[2].powi(2) - x[1].powi(2) - x[0].powi(2),
                },
                TestConstraint::Ineq {
                    expr: "x[0]**2 + x[1]**2 + x[2]**2 - 4.0".to_string(),
                    fun: |x| x[0].powi(2) + x[1].powi(2) + x[2].powi(2) - 4.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS34
        TestCase {
            name: "HS34".to_string(),
            objective_expr: "x[0]".to_string(),
            objective_fn: |x| x[0],
            x0: vec![0.0, 1.05, 2.9],
            bounds: Some(vec![(0.0, 100.0), (0.0, 100.0), (0.0, 10.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[1] - exp(x[0])".to_string(),
                    fun: |x| x[1] - x[0].exp(),
                },
                TestConstraint::Ineq {
                    expr: "x[2] - exp(x[1])".to_string(),
                    fun: |x| x[2] - x[1].exp(),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS35
        TestCase {
            name: "HS35".to_string(),
            objective_expr: "9.0 - 8.0 * x[0] - 6.0 * x[1] - 4.0 * x[2] + 2.0 * x[0]**2 + 2.0 * x[1]**2 + x[2]**2 + 2.0 * x[0] * x[1] + 2.0 * x[0] * x[2]".to_string(),
            objective_fn: |x| 9.0 - 8.0 * x[0] - 6.0 * x[1] - 4.0 * x[2] + 2.0 * x[0].powi(2) + 2.0 * x[1].powi(2) + x[2].powi(2) + 2.0 * x[0] * x[1] + 2.0 * x[0] * x[2],
            x0: vec![0.5, 0.5, 0.5],
            bounds: Some(vec![(0.0, 1e10), (0.0, 1e10), (0.0, 1e10)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "3.0 - x[0] - x[1] - 2.0 * x[2]".to_string(),
                    fun: |x| 3.0 - x[0] - x[1] - 2.0 * x[2],
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS36
        TestCase {
            name: "HS36".to_string(),
            objective_expr: "-x[0] * x[1] * x[2]".to_string(),
            objective_fn: |x| -x[0] * x[1] * x[2],
            x0: vec![10.0, 10.0, 10.0],
            bounds: Some(vec![(0.0, 20.0), (0.0, 11.0), (0.0, 42.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "72.0 - x[0] - 2.0 * x[1] - 2.0 * x[2]".to_string(),
                    fun: |x| 72.0 - x[0] - 2.0 * x[1] - 2.0 * x[2],
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS37
        TestCase {
            name: "HS37".to_string(),
            objective_expr: "-x[0] * x[1] * x[2]".to_string(),
            objective_fn: |x| -x[0] * x[1] * x[2],
            x0: vec![10.0, 10.0, 10.0],
            bounds: Some(vec![(0.0, 42.0), (0.0, 42.0), (0.0, 42.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "72.0 - x[0] - 2.0 * x[1] - 2.0 * x[2]".to_string(),
                    fun: |x| 72.0 - x[0] - 2.0 * x[1] - 2.0 * x[2],
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS38
        TestCase {
            name: "HS38".to_string(),
            objective_expr: "100.0 * (x[1] - x[0]**2)**2 + (1.0 - x[0])**2 + 90.0 * (x[3] - x[2]**2)**2 + (1.0 - x[2])**2 + 10.1 * ((x[1] - 1.0)**2 + (x[3] - 1.0)**2) + 19.8 * (x[1] - 1.0) * (x[3] - 1.0)".to_string(),
            objective_fn: |x| 100.0 * (x[1] - x[0].powi(2)).powi(2) + (1.0 - x[0]).powi(2) + 90.0 * (x[3] - x[2].powi(2)).powi(2) + (1.0 - x[2]).powi(2) + 10.1 * ((x[1] - 1.0).powi(2) + (x[3] - 1.0).powi(2)) + 19.8 * (x[1] - 1.0) * (x[3] - 1.0),
            x0: vec![-3.0, -1.0, -3.0, -1.0],
            bounds: Some(vec![(-10.0, 10.0), (-10.0, 10.0), (-10.0, 10.0), (-10.0, 10.0)]),
            constraints: vec![],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS39
        TestCase {
            name: "HS39".to_string(),
            objective_expr: "-x[0]".to_string(),
            objective_fn: |x| -x[0],
            x0: vec![2.0, 2.0, 2.0, 2.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[1] - x[0]**3 - x[2]**2".to_string(),
                    fun: |x| x[1] - x[0].powi(3) - x[2].powi(2),
                },
                TestConstraint::Eq {
                    expr: "x[0]**2 - x[1] - x[3]**2".to_string(),
                    fun: |x| x[0].powi(2) - x[1] - x[3].powi(2),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS40
        TestCase {
            name: "HS40".to_string(),
            objective_expr: "-x[0] * x[1] * x[2] * x[3]".to_string(),
            objective_fn: |x| -x[0] * x[1] * x[2] * x[3],
            x0: vec![0.8, 0.8, 0.8, 0.8],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0]**3 + x[1]**2 - 1.0".to_string(),
                    fun: |x| x[0].powi(3) + x[1].powi(2) - 1.0,
                },
                TestConstraint::Eq {
                    expr: "x[0]**2 * x[3] - x[2]".to_string(),
                    fun: |x| x[0].powi(2) * x[3] - x[2],
                },
                TestConstraint::Eq {
                    expr: "x[3]**2 - x[1]".to_string(),
                    fun: |x| x[3].powi(2) - x[1],
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS41
        TestCase {
            name: "HS41".to_string(),
            objective_expr: "2.0 - x[0] * x[1] * x[2] * x[3] / 120.0".to_string(),
            objective_fn: |x| 2.0 - x[0] * x[1] * x[2] * x[3] / 120.0,
            x0: vec![2.0, 2.0, 2.0, 2.0],
            bounds: Some(vec![(0.0, 1.0), (0.0, 2.0), (0.0, 2.0), (0.0, 1.0)]),
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + 2.0 * x[1] + 2.0 * x[2] + x[3] - 10.0".to_string(),
                    fun: |x| x[0] + 2.0 * x[1] + 2.0 * x[2] + x[3] - 10.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS42
        TestCase {
            name: "HS42".to_string(),
            objective_expr: "(x[0] - 1.0)**2 + (x[1] - 2.0)**2 + (x[2] - 3.0)**2 + (x[3] - 4.0)**2".to_string(),
            objective_fn: |x| (x[0] - 1.0).powi(2) + (x[1] - 2.0).powi(2) + (x[2] - 3.0).powi(2) + (x[3] - 4.0).powi(2),
            x0: vec![2.0, 2.0, 2.0, 2.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] - 2.0".to_string(),
                    fun: |x| x[0] - 2.0,
                },
                TestConstraint::Eq {
                    expr: "x[2]**2 + x[3]**2 - 2.0".to_string(),
                    fun: |x| x[2].powi(2) + x[3].powi(2) - 2.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS43
        TestCase {
            name: "HS43".to_string(),
            objective_expr: "x[0]**2 + x[1]**2 + 2.0 * x[2]**2 + x[3]**2 - 5.0 * x[0] - 5.0 * x[1] - 21.0 * x[2] + 7.0 * x[3]".to_string(),
            objective_fn: |x| x[0].powi(2) + x[1].powi(2) + 2.0 * x[2].powi(2) + x[3].powi(2) - 5.0 * x[0] - 5.0 * x[1] - 21.0 * x[2] + 7.0 * x[3],
            x0: vec![0.0, 0.0, 0.0, 0.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "8.0 - (x[0]**2 + x[1]**2 + x[2]**2 + x[3]**2 + x[0] - x[1] + x[2] - x[3])".to_string(),
                    fun: |x| 8.0 - (x[0].powi(2) + x[1].powi(2) + x[2].powi(2) + x[3].powi(2) + x[0] - x[1] + x[2] - x[3]),
                },
                TestConstraint::Ineq {
                    expr: "10.0 - (x[0]**2 + 2.0 * x[1]**2 + x[2]**2 + 2.0 * x[3]**2 - x[0] - x[3])".to_string(),
                    fun: |x| 10.0 - (x[0].powi(2) + 2.0 * x[1].powi(2) + x[2].powi(2) + 2.0 * x[3].powi(2) - x[0] - x[3]),
                },
                TestConstraint::Ineq {
                    expr: "5.0 - (2.0 * x[0]**2 + x[1]**2 + x[2]**2 + 2.0 * x[0] - x[1] - x[3])".to_string(),
                    fun: |x| 5.0 - (2.0 * x[0].powi(2) + x[1].powi(2) + x[2].powi(2) + 2.0 * x[0] - x[1] - x[3]),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS44
        TestCase {
            name: "HS44".to_string(),
            objective_expr: "x[0] - x[1] - x[2] - x[0] * x[2] + x[1] * x[2] + x[0] * x[3]".to_string(),
            objective_fn: |x| x[0] - x[1] - x[2] - x[0] * x[2] + x[1] * x[2] + x[0] * x[3],
            x0: vec![0.0, 0.0, 0.0, 0.0],
            bounds: Some(vec![(0.0, 1e10), (0.0, 1e10), (0.0, 1e10), (0.0, 1e10)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "8.0 - x[0] - 2.0 * x[1]".to_string(),
                    fun: |x| 8.0 - x[0] - 2.0 * x[1],
                },
                TestConstraint::Ineq {
                    expr: "12.0 - 4.0 * x[0] - x[1]".to_string(),
                    fun: |x| 12.0 - 4.0 * x[0] - x[1],
                },
                TestConstraint::Ineq {
                    expr: "12.0 - 3.0 * x[0] - 4.0 * x[1]".to_string(),
                    fun: |x| 12.0 - 3.0 * x[0] - 4.0 * x[1],
                },
                TestConstraint::Ineq {
                    expr: "8.0 - 2.0 * x[2] - x[3]".to_string(),
                    fun: |x| 8.0 - 2.0 * x[2] - x[3],
                },
                TestConstraint::Ineq {
                    expr: "8.0 - x[2] - 2.0 * x[3]".to_string(),
                    fun: |x| 8.0 - x[2] - 2.0 * x[3],
                },
                TestConstraint::Ineq {
                    expr: "5.0 - x[2] - x[3]".to_string(),
                    fun: |x| 5.0 - x[2] - x[3],
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS45
        TestCase {
            name: "HS45".to_string(),
            objective_expr: "2.0 - x[0] * x[1] * x[2] * x[3] * x[4] / 120.0".to_string(),
            objective_fn: |x| 2.0 - x[0] * x[1] * x[2] * x[3] * x[4] / 120.0,
            x0: vec![2.0, 2.0, 2.0, 2.0, 2.0],
            bounds: Some(vec![(0.0, 1.0), (0.0, 2.0), (0.0, 3.0), (0.0, 4.0), (0.0, 5.0)]),
            constraints: vec![],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS46
        TestCase {
            name: "HS46".to_string(),
            objective_expr: "(x[0] - x[1])**2 + (x[1] - x[2])**2 + (x[2] - x[3])**4 + (x[3] - x[4])**2".to_string(),
            objective_fn: |x| (x[0] - x[1]).powi(2) + (x[1] - x[2]).powi(2) + (x[2] - x[3]).powi(4) + (x[3] - x[4]).powi(2),
            x0: vec![0.5, 0.5, 0.5, 0.5, 0.5],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + x[1]**2 + x[2]**3 - (3.0 + 1.4142135623730951)".to_string(),
                    fun: |x| x[0] + x[1].powi(2) + x[2].powi(3) - (3.0 + 2.0f64.sqrt()),
                },
                TestConstraint::Eq {
                    expr: "x[1] - x[2]**2 + x[3] - 1.0".to_string(),
                    fun: |x| x[1] - x[2].powi(2) + x[3] - 1.0,
                },
                TestConstraint::Eq {
                    expr: "x[0] * x[4] - 1.0".to_string(),
                    fun: |x| x[0] * x[4] - 1.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS47
        TestCase {
            name: "HS47".to_string(),
            objective_expr: "(x[0] - 1.0)**2 + (x[0] - x[1])**2 + (x[1] - x[2])**2 + (x[2] - x[3])**4 + (x[3] - x[4])**4".to_string(),
            objective_fn: |x| (x[0] - 1.0).powi(2) + (x[0] - x[1]).powi(2) + (x[1] - x[2]).powi(2) + (x[2] - x[3]).powi(4) + (x[3] - x[4]).powi(4),
            x0: vec![2.0, 2.0, 2.0, 2.0, 2.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + x[1]**2 + x[2]**3 - (3.0 + 1.4142135623730951)".to_string(),
                    fun: |x| x[0] + x[1].powi(2) + x[2].powi(3) - (3.0 + 2.0f64.sqrt()),
                },
                TestConstraint::Eq {
                    expr: "x[1] - x[2]**2 + x[3] - 1.0".to_string(),
                    fun: |x| x[1] - x[2].powi(2) + x[3] - 1.0,
                },
                TestConstraint::Eq {
                    expr: "x[0] * x[4] - 1.0".to_string(),
                    fun: |x| x[0] * x[4] - 1.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS48
        TestCase {
            name: "HS48".to_string(),
            objective_expr: "(x[0] - 1.0)**2 + (x[1] - x[2])**2 + (x[3] - x[4])**2".to_string(),
            objective_fn: |x| (x[0] - 1.0).powi(2) + (x[1] - x[2]).powi(2) + (x[3] - x[4]).powi(2),
            x0: vec![3.0, 5.0, -3.0, 2.0, -2.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + x[1] + x[2] + x[3] + x[4] - 5.0".to_string(),
                    fun: |x| x[0] + x[1] + x[2] + x[3] + x[4] - 5.0,
                },
                TestConstraint::Eq {
                    expr: "x[2] + x[3]**2 + x[4] - 3.0".to_string(),
                    fun: |x| x[2] + x[3].powi(2) + x[4] - 3.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS49
        TestCase {
            name: "HS49".to_string(),
            objective_expr: "(x[0] - x[1])**2 + (x[1] - x[2])**2 + (x[2] - x[3])**4 + (x[3] - x[4])**4".to_string(),
            objective_fn: |x| (x[0] - x[1]).powi(2) + (x[1] - x[2]).powi(2) + (x[2] - x[3]).powi(4) + (x[3] - x[4]).powi(4),
            x0: vec![10.0, 10.0, 10.0, 10.0, 10.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + x[1]**2 + x[2]**3 - (3.0 + 1.4142135623730951)".to_string(),
                    fun: |x| x[0] + x[1].powi(2) + x[2].powi(3) - (3.0 + 2.0f64.sqrt()),
                },
                TestConstraint::Eq {
                    expr: "x[1] - x[2]**2 + x[3] - (1.0 + 2.0 * 1.4142135623730951)".to_string(),
                    fun: |x| x[1] - x[2].powi(2) + x[3] - (1.0 + 2.0 * 2.0f64.sqrt()),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS50
        TestCase {
            name: "HS50".to_string(),
            objective_expr: "(x[0] - x[1])**2 + (x[1] - x[2])**2 + (x[2] - x[3])**2 + (x[3] - x[4])**2 + (x[4] - x[0])**2".to_string(),
            objective_fn: |x| (x[0] - x[1]).powi(2) + (x[1] - x[2]).powi(2) + (x[2] - x[3]).powi(2) + (x[3] - x[4]).powi(2) + (x[4] - x[0]).powi(2),
            x0: vec![35.0, -31.0, 11.0, 5.0, -5.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + 2.0 * x[1] + 3.0 * x[2] - 6.0".to_string(),
                    fun: |x| x[0] + 2.0 * x[1] + 3.0 * x[2] - 6.0,
                },
                TestConstraint::Eq {
                    expr: "x[1] + 2.0 * x[2] + 3.0 * x[3] - 6.0".to_string(),
                    fun: |x| x[1] + 2.0 * x[2] + 3.0 * x[3] - 6.0,
                },
                TestConstraint::Eq {
                    expr: "x[2] + 2.0 * x[3] + 3.0 * x[4] - 6.0".to_string(),
                    fun: |x| x[2] + 2.0 * x[3] + 3.0 * x[4] - 6.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS51
        TestCase {
            name: "HS51".to_string(),
            objective_expr: "(x[0] - 1.0)**2 + (x[1] - 1.0)**2 + (x[2] - 1.0)**2 + (x[3] - 1.0)**2 + (x[4] - 1.0)**2".to_string(),
            objective_fn: |x| (x[0] - 1.0).powi(2) + (x[1] - 1.0).powi(2) + (x[2] - 1.0).powi(2) + (x[3] - 1.0).powi(2) + (x[4] - 1.0).powi(2),
            x0: vec![2.5, 0.5, 2.0, -1.0, 0.5],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + 3.0 * x[1]".to_string(),
                    fun: |x| x[0] + 3.0 * x[1],
                },
                TestConstraint::Eq {
                    expr: "x[2] + x[3] - 2.0 * x[4]".to_string(),
                    fun: |x| x[2] + x[3] - 2.0 * x[4],
                },
                TestConstraint::Eq {
                    expr: "x[1] - x[4]".to_string(),
                    fun: |x| x[1] - x[4],
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS52
        TestCase {
            name: "HS52".to_string(),
            objective_expr: "(4.0 * x[0] - x[1])**2 + (x[1] + x[2] - 2.0)**2 + (x[3] - 1.0)**2 + (x[4] - 1.0)**2".to_string(),
            objective_fn: |x| (4.0 * x[0] - x[1]).powi(2) + (x[1] + x[2] - 2.0).powi(2) + (x[3] - 1.0).powi(2) + (x[4] - 1.0).powi(2),
            x0: vec![2.0, 2.0, 2.0, 2.0, 2.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + 3.0 * x[1]".to_string(),
                    fun: |x| x[0] + 3.0 * x[1],
                },
                TestConstraint::Eq {
                    expr: "x[2] + x[3] - 2.0 * x[4]".to_string(),
                    fun: |x| x[2] + x[3] - 2.0 * x[4],
                },
                TestConstraint::Eq {
                    expr: "x[1] - x[4]".to_string(),
                    fun: |x| x[1] - x[4],
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS53
        TestCase {
            name: "HS53".to_string(),
            objective_expr: "(x[0] - x[1])**2 + (x[1] + x[2] - 2.0)**2 + (x[3] - 1.0)**2 + (x[4] - 1.0)**2".to_string(),
            objective_fn: |x| (x[0] - x[1]).powi(2) + (x[1] + x[2] - 2.0).powi(2) + (x[3] - 1.0).powi(2) + (x[4] - 1.0).powi(2),
            x0: vec![2.0, 2.0, 2.0, 2.0, 2.0],
            bounds: Some(vec![(-10.0, 10.0), (-10.0, 10.0), (-10.0, 10.0), (-10.0, 10.0), (-10.0, 10.0)]),
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + 3.0 * x[1]".to_string(),
                    fun: |x| x[0] + 3.0 * x[1],
                },
                TestConstraint::Eq {
                    expr: "x[2] + x[3] - 2.0 * x[4]".to_string(),
                    fun: |x| x[2] + x[3] - 2.0 * x[4],
                },
                TestConstraint::Eq {
                    expr: "x[1] - x[4]".to_string(),
                    fun: |x| x[1] - x[4],
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS54
        TestCase {
            name: "HS54".to_string(),
            objective_expr: "(x[0]-1)**2 + (x[1]-2)**2 + (x[2]-3)**2 + (x[3]-4)**2 + (x[4]-5)**2 + (x[5]-6)**2".to_string(),
            objective_fn: |x| (x[0]-1.0).powi(2) + (x[1]-2.0).powi(2) + (x[2]-3.0).powi(2) + (x[3]-4.0).powi(2) + (x[4]-5.0).powi(2) + (x[5]-6.0).powi(2),
            x0: vec![1.1, 2.1, 3.1, 4.1, 5.1, 6.1],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + x[1] + x[2] + x[3] + x[4] + x[5] - 21.0".to_string(),
                    fun: |x| x[0] + x[1] + x[2] + x[3] + x[4] + x[5] - 21.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS55
        TestCase {
            name: "HS55".to_string(),
            objective_expr: "x[0] + 2.0 * x[1] + 4.0 * x[4] + exp(x[0] * x[3])".to_string(),
            objective_fn: |x| x[0] + 2.0 * x[1] + 4.0 * x[4] + (x[0] * x[3]).exp(),
            x0: vec![0.0, 4.0/3.0, 5.0/3.0, 1.0, 2.0/3.0, 1.0/3.0],
            bounds: Some(vec![(0.0, 1e10), (0.0, 1e10), (0.0, 1e10), (0.0, 1e10), (0.0, 1e10), (0.0, 1e10)]),
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + 2.0 * x[1] + 5.0 * x[4] - 6.0".to_string(),
                    fun: |x| x[0] + 2.0 * x[1] + 5.0 * x[4] - 6.0,
                },
                TestConstraint::Eq {
                    expr: "x[0] + x[1] + x[2] - 3.0".to_string(),
                    fun: |x| x[0] + x[1] + x[2] - 3.0,
                },
                TestConstraint::Eq {
                    expr: "x[3] + x[4] + x[5] - 2.0".to_string(),
                    fun: |x| x[3] + x[4] + x[5] - 2.0,
                },
                TestConstraint::Eq {
                    expr: "x[0] + x[3] - 1.0".to_string(),
                    fun: |x| x[0] + x[3] - 1.0,
                },
                TestConstraint::Eq {
                    expr: "x[1] + x[4] - 2.0".to_string(),
                    fun: |x| x[1] + x[4] - 2.0,
                },
                TestConstraint::Eq {
                    expr: "x[2] + x[5] - 2.0".to_string(),
                    fun: |x| x[2] + x[5] - 2.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS56
        TestCase {
            name: "HS56".to_string(),
            objective_expr: "-x[0] * x[1] * x[2] * x[3] * x[4] * x[5] * x[6]".to_string(),
            objective_fn: |x| -x[0] * x[1] * x[2] * x[3] * x[4] * x[5] * x[6],
            x0: vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0]**2 + x[1]**2 + x[2]**2 + x[3]**2 + x[4]**2 + x[5]**2 + x[6]**2 - 1.0".to_string(),
                    fun: |x| x[0].powi(2) + x[1].powi(2) + x[2].powi(2) + x[3].powi(2) + x[4].powi(2) + x[5].powi(2) + x[6].powi(2) - 1.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS57
        TestCase {
            name: "HS57".to_string(),
            objective_expr: "0.4 * (x[0]**2 + x[1]**2) + x[0] + x[1]".to_string(),
            objective_fn: |x| 0.4 * (x[0].powi(2) + x[1].powi(2)) + x[0] + x[1],
            x0: vec![0.42, 0.42],
            bounds: None,
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[0]**2 + x[1]**2 - 1.0".to_string(),
                    fun: |x| x[0].powi(2) + x[1].powi(2) - 1.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS58
        TestCase {
            name: "HS58".to_string(),
            objective_expr: "x[0]**2 + x[1]**2".to_string(),
            objective_fn: |x| x[0].powi(2) + x[1].powi(2),
            x0: vec![1.0, 1.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0]**2 - x[1] - 1.0".to_string(),
                    fun: |x| x[0].powi(2) - x[1] - 1.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS59
        TestCase {
            name: "HS59".to_string(),
            objective_expr: "x[0]**2 + x[1]**2".to_string(),
            objective_fn: |x| x[0].powi(2) + x[1].powi(2),
            x0: vec![2.0, 2.0],
            bounds: Some(vec![(0.0, 1e10), (0.0, 1e10)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[0] * x[1] - 1.0".to_string(),
                    fun: |x| x[0] * x[1] - 1.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS60
        TestCase {
            name: "HS60".to_string(),
            objective_expr: "(x[0]-1.0)**2 + (x[0]-x[1])**2 + (x[1]-x[2])**4".to_string(),
            objective_fn: |x| (x[0]-1.0).powi(2) + (x[0]-x[1]).powi(2) + (x[1]-x[2]).powi(4),
            x0: vec![2.0, 2.0, 2.0],
            bounds: Some(vec![(-10.0, 10.0), (-10.0, 10.0), (-10.0, 10.0)]),
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] * (1.0 + x[1]**2) + x[2]**4 - (4.0 + 3.0 * sqrt(2.0))".to_string(),
                    fun: |x| x[0] * (1.0 + x[1].powi(2)) + x[2].powi(4) - (4.0 + 3.0 * 2.0f64.sqrt()),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS61
        TestCase {
            name: "HS61".to_string(),
            objective_expr: "4.0 * x[0]**2 + 2.0 * x[1]**2 + 4.0 * x[0] * x[1] + 2.0 * x[0] + 1.0".to_string(),
            objective_fn: |x| 4.0 * x[0].powi(2) + 2.0 * x[1].powi(2) + 4.0 * x[0] * x[1] + 2.0 * x[0] + 1.0,
            x0: vec![0.0, 0.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[0] * x[1] - x[0] - x[1] + 1.5".to_string(),
                    fun: |x| x[0] * x[1] - x[0] - x[1] + 1.5,
                },
                TestConstraint::Ineq {
                    expr: "x[0] * x[1] + 10.0".to_string(),
                    fun: |x| x[0] * x[1] + 10.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS62
        TestCase {
            name: "HS62".to_string(),
            objective_expr: "-x[0] * x[1] * x[2]".to_string(),
            objective_fn: |x| -x[0] * x[1] * x[2],
            x0: vec![10.0, 10.0, 10.0],
            bounds: Some(vec![(0.0, 42.0), (0.0, 42.0), (0.0, 42.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "72.0 - (x[0] + 2.0 * x[1] + 2.0 * x[2])".to_string(),
                    fun: |x| 72.0 - (x[0] + 2.0 * x[1] + 2.0 * x[2]),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: Some(2e-4),
        },
        // HS63
        TestCase {
            name: "HS63".to_string(),
            objective_expr: "1000.0 - x[0]**2 - 2.0 * x[1]**2 - x[2]**2 - x[0]*x[1] - x[0]*x[2]".to_string(),
            objective_fn: |x| 1000.0 - x[0].powi(2) - 2.0 * x[1].powi(2) - x[2].powi(2) - x[0]*x[1] - x[0]*x[2],
            x0: vec![2.0, 2.0, 2.0],
            bounds: Some(vec![(0.0, 1e10), (0.0, 1e10), (0.0, 1e10)]),
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0]**2 + x[1]**2 + x[2]**2 - 25.0".to_string(),
                    fun: |x| x[0].powi(2) + x[1].powi(2) + x[2].powi(2) - 25.0,
                },
                TestConstraint::Eq {
                    expr: "8.0 * x[0] + 14.0 * x[1] + 7.0 * x[2] - 56.0".to_string(),
                    fun: |x| 8.0 * x[0] + 14.0 * x[1] + 7.0 * x[2] - 56.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS64
        TestCase {
            name: "HS64".to_string(),
            objective_expr: "5.0 * x[0] + 50000.0 / x[0] + 20.0 * x[1] + 72000.0 / x[1] + 10.0 * x[2] + 144000.0 / x[2]".to_string(),
            objective_fn: |x| 5.0 * x[0] + 50000.0 / x[0] + 20.0 * x[1] + 72000.0 / x[1] + 10.0 * x[2] + 144000.0 / x[2],
            x0: vec![1.0, 1.0, 1.0],
            bounds: Some(vec![(1e-5, 1e6), (1e-5, 1e6), (1e-5, 1e6)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "1.0 - (4.0 / x[0] + 32.0 / x[1] + 120.0 / x[2])".to_string(),
                    fun: |x| 1.0 - (4.0 / x[0] + 32.0 / x[1] + 120.0 / x[2]),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: Some(1e-3),
        },
        // HS65
        TestCase {
            name: "HS65".to_string(),
            objective_expr: "(x[0] - 5.0)**2 + (x[1] - 5.0)**2 + (x[2] - 5.0)**2".to_string(),
            objective_fn: |x| (x[0] - 5.0).powi(2) + (x[1] - 5.0).powi(2) + (x[2] - 5.0).powi(2),
            x0: vec![0.0, 0.0, 0.0],
            bounds: Some(vec![(-4.5, 4.5), (-4.5, 4.5), (-4.5, 4.5)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "25.0 - (x[0]**2 + x[1]**2 + x[2]**2)".to_string(),
                    fun: |x| 25.0 - (x[0].powi(2) + x[1].powi(2) + x[2].powi(2)),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS66
        TestCase {
            name: "HS66".to_string(),
            objective_expr: "0.2 * x[2] - 0.8 * x[0]".to_string(),
            objective_fn: |x| 0.2 * x[2] - 0.8 * x[0],
            x0: vec![0.0, 1.05, 2.9],
            bounds: Some(vec![(0.0, 100.0), (0.0, 100.0), (0.0, 10.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[1] - exp(x[0])".to_string(),
                    fun: |x| x[1] - x[0].exp(),
                },
                TestConstraint::Ineq {
                    expr: "x[2] - exp(x[1])".to_string(),
                    fun: |x| x[2] - x[1].exp(),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS67
        TestCase {
            name: "HS67".to_string(),
            objective_expr: "(x[0]-1.0)**2 + (x[1]-2.0)**2 + (x[2]-3.0)**2".to_string(),
            objective_fn: |x| (x[0]-1.0).powi(2) + (x[1]-2.0).powi(2) + (x[2]-3.0).powi(2),
            x0: vec![1.0, 1.0, 1.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + x[1] + x[2] - 1.0".to_string(),
                    fun: |x| x[0] + x[1] + x[2] - 1.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS68
        TestCase {
            name: "HS68".to_string(),
            objective_expr: "x[0]**2 + x[1]**2 + x[2]**2 + x[3]**2".to_string(),
            objective_fn: |x| x[0].powi(2) + x[1].powi(2) + x[2].powi(2) + x[3].powi(2),
            x0: vec![0.5, 1.5, 0.5, 1.5],
            bounds: Some(vec![(0.0, 1e10), (0.0, 1e10), (0.0, 1e10), (0.0, 1e10)]),
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + x[1] + x[2] + x[3] - 4.0".to_string(),
                    fun: |x| x[0] + x[1] + x[2] + x[3] - 4.0,
                },
                TestConstraint::Eq {
                    expr: "x[0] * x[1] * x[2] * x[3] - 1.0".to_string(),
                    fun: |x| x[0] * x[1] * x[2] * x[3] - 1.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS69
        TestCase {
            name: "HS69".to_string(),
            objective_expr: "(x[0]-1.0)**2 + (x[1]-2.0)**2 + (x[2]-3.0)**2 + (x[3]-4.0)**2".to_string(),
            objective_fn: |x| (x[0]-1.0).powi(2) + (x[1]-2.0).powi(2) + (x[2]-3.0).powi(2) + (x[3]-4.0).powi(2),
            x0: vec![2.5, 2.5, 2.5, 2.5],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + x[1] + x[2] + x[3] - 10.0".to_string(),
                    fun: |x| x[0] + x[1] + x[2] + x[3] - 10.0,
                },
                TestConstraint::Eq {
                    expr: "x[0]**2 + x[1]**2 + x[2]**2 + x[3]**2 - 30.0".to_string(),
                    fun: |x| x[0].powi(2) + x[1].powi(2) + x[2].powi(2) + x[3].powi(2) - 30.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS70
        TestCase {
            name: "HS70".to_string(),
            objective_expr: "x[0] + x[1] + x[2] + x[3]".to_string(),
            objective_fn: |x| x[0] + x[1] + x[2] + x[3],
            x0: vec![1.0, 1.0, 1.0, 1.0],
            bounds: Some(vec![(1.0, 1e10), (1.0, 1e10), (1.0, 1e10), (1.0, 1e10)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[0] * x[1] * x[2] * x[3] - 10.0".to_string(),
                    fun: |x| x[0] * x[1] * x[2] * x[3] - 10.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS72
        TestCase {
            name: "HS72".to_string(),
            objective_expr: "x[0]**2 + x[1]**2 + 2.0*x[2]**2 + x[3]**2 - 5.0*x[0] - 5.0*x[1] - 21.0*x[2] + 7.0*x[3]".to_string(),
            objective_fn: |x| x[0].powi(2) + x[1].powi(2) + 2.0*x[2].powi(2) + x[3].powi(2) - 5.0*x[0] - 5.0*x[1] - 21.0*x[2] + 7.0*x[3],
            x0: vec![0.0, 0.0, 0.0, 0.0],
            bounds: Some(vec![(0.0, 1e10), (0.0, 1e10), (0.0, 1e10), (0.0, 1e10)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "8.0 - (2.0*x[0]**2 + x[1]**2 + x[2]**2 + 2.0*x[0] + x[1] + x[2])".to_string(),
                    fun: |x| 8.0 - (2.0*x[0].powi(2) + x[1].powi(2) + x[2].powi(2) + 2.0*x[0] + x[1] + x[2]),
                },
                TestConstraint::Ineq {
                    expr: "10.0 - (x[0]**2 + x[1]**2 + x[2]**2 + x[3]**2 + x[0] - x[1] + x[2] - x[3])".to_string(),
                    fun: |x| 10.0 - (x[0].powi(2) + x[1].powi(2) + x[2].powi(2) + x[3].powi(2) + x[0] - x[1] + x[2] - x[3]),
                },
                TestConstraint::Ineq {
                    expr: "5.0 - (x[0]**2 + 2.0*x[1]**2 + x[2]**2 + 2.0*x[3]**2 - x[0] - x[3])".to_string(),
                    fun: |x| 5.0 - (x[0].powi(2) + 2.0*x[1].powi(2) + x[2].powi(2) + 2.0*x[3].powi(2) - x[0] - x[3]),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS71
        TestCase {
            name: "HS71".to_string(),
            objective_expr: "x[0] * x[3] * (x[0] + x[1] + x[2]) + x[2]".to_string(),
            objective_fn: |x| x[0] * x[3] * (x[0] + x[1] + x[2]) + x[2],
            x0: vec![1.0, 5.0, 5.0, 1.0],
            bounds: Some(vec![(1.0, 5.0), (1.0, 5.0), (1.0, 5.0), (1.0, 5.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[0] * x[1] * x[2] * x[3] - 25.0".to_string(),
                    fun: |x| x[0] * x[1] * x[2] * x[3] - 25.0,
                },
                TestConstraint::Eq {
                    expr: "x[0]**2 + x[1]**2 + x[2]**2 + x[3]**2 - 40.0".to_string(),
                    fun: |x| x[0].powi(2) + x[1].powi(2) + x[2].powi(2) + x[3].powi(2) - 40.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS74
        TestCase {
            name: "HS74".to_string(),
            objective_expr: "3.0*x[0] + 1e-6*x[0]**3 + 2.0*x[1] + (2.0/3.0)*1e-6*x[1]**3".to_string(),
            objective_fn: |x| 3.0*x[0] + 1e-6*x[0].powi(3) + 2.0*x[1] + (2.0/3.0)*1e-6*x[1].powi(3),
            x0: vec![10.0, 10.0, 10.0, 10.0],
            bounds: Some(vec![(0.0, 1200.0), (0.0, 1200.0), (-0.55, 0.55), (-0.55, 0.55)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[3] - x[2] + 0.55".to_string(),
                    fun: |x| x[3] - x[2] + 0.55,
                },
                TestConstraint::Ineq {
                    expr: "x[2] - x[3] + 0.55".to_string(),
                    fun: |x| x[2] - x[3] + 0.55,
                },
                TestConstraint::Eq {
                    expr: "x[0] - 1000.0 * sin(-x[2]-0.25) - 1000.0 * sin(-x[3]-0.25) - 894.8".to_string(),
                    fun: |x| x[0] - 1000.0 * (-x[2]-0.25).sin() - 1000.0 * (-x[3]-0.25).sin() - 894.8,
                },
                TestConstraint::Eq {
                    expr: "x[1] - 1000.0 * sin(x[2]-0.25) - 1000.0 * sin(x[2]-x[3]-0.25) - 894.8".to_string(),
                    fun: |x| x[1] - 1000.0 * (x[2]-0.25).sin() - 1000.0 * (x[2]-x[3]-0.25).sin() - 894.8,
                },
                TestConstraint::Eq {
                    expr: "1000.0 * sin(x[3]-0.25) + 1000.0 * sin(x[3]-x[2]-0.25) + 1294.8".to_string(),
                    fun: |x| 1000.0 * (x[3]-0.25).sin() + 1000.0 * (x[3]-x[2]-0.25).sin() + 1294.8,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: Some(1e-2),
        },
        // HS75
        TestCase {
            name: "HS75".to_string(),
            objective_expr: "3.0*x[0] + 1e-6*x[0]**3 + 2.0*x[1] + (2.0/3.0)*1e-6*x[1]**3".to_string(),
            objective_fn: |x| 3.0*x[0] + 1e-6*x[0].powi(3) + 2.0*x[1] + (2.0/3.0)*1e-6*x[1].powi(3),
            x0: vec![10.0, 10.0, 10.0, 10.0],
            bounds: Some(vec![(0.0, 1200.0), (0.0, 1200.0), (-0.55, 0.55), (-0.55, 0.55)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[3] - x[2] + 0.55".to_string(),
                    fun: |x| x[3] - x[2] + 0.55,
                },
                TestConstraint::Ineq {
                    expr: "x[2] - x[3] + 0.55".to_string(),
                    fun: |x| x[2] - x[3] + 0.55,
                },
                TestConstraint::Ineq {
                    expr: "x[0] - 1000.0 * sin(-x[2]-0.25) - 1000.0 * sin(-x[3]-0.25) - 894.8".to_string(),
                    fun: |x| x[0] - 1000.0 * (-x[2]-0.25).sin() - 1000.0 * (-x[3]-0.25).sin() - 894.8,
                },
                TestConstraint::Ineq {
                    expr: "x[1] - 1000.0 * sin(x[2]-0.25) - 1000.0 * sin(x[2]-x[3]-0.25) - 894.8".to_string(),
                    fun: |x| x[1] - 1000.0 * (x[2]-0.25).sin() - 1000.0 * (x[2]-x[3]-0.25).sin() - 894.8,
                },
                TestConstraint::Ineq {
                    expr: "1000.0 * sin(x[3]-0.25) + 1000.0 * sin(x[3]-x[2]-0.25) + 1294.8".to_string(),
                    fun: |x| 1000.0 * (x[3]-0.25).sin() + 1000.0 * (x[3]-x[2]-0.25).sin() + 1294.8,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS73
        TestCase {
            name: "HS73".to_string(),
            objective_expr: "2.0 * x[0] + 3.0 * x[1] + x[2] + x[3] + x[4]".to_string(),
            objective_fn: |x| 2.0 * x[0] + 3.0 * x[1] + x[2] + x[3] + x[4],
            x0: vec![1.0, 1.0, 1.0, 1.0, 1.0],
            bounds: Some(vec![(0.0, 1e10), (0.0, 1e10), (0.0, 1e10), (0.0, 1e10), (0.0, 1e10)]),
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + 2.0 * x[1] + x[2] + x[3] + x[4] - 10.0".to_string(),
                    fun: |x| x[0] + 2.0 * x[1] + x[2] + x[3] + x[4] - 10.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS76
        TestCase {
            name: "HS76".to_string(),
            objective_expr: "x[0]**2 + 0.5 * x[1]**2 + x[2]**2 + 0.5 * x[3]**2 - x[0]*x[2] + x[2]*x[3] - x[0] - 3.0*x[1] + x[2] - x[3]".to_string(),
            objective_fn: |x| x[0].powi(2) + 0.5 * x[1].powi(2) + x[2].powi(2) + 0.5 * x[3].powi(2) - x[0]*x[2] + x[2]*x[3] - x[0] - 3.0*x[1] + x[2] - x[3],
            x0: vec![0.5, 0.5, 0.5, 0.5],
            bounds: Some(vec![(0.0, 1e10), (0.0, 1e10), (0.0, 1e10), (0.0, 1e10)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "5.0 - (x[0] + 2.0 * x[1] + x[2] + x[3])".to_string(),
                    fun: |x| 5.0 - (x[0] + 2.0 * x[1] + x[2] + x[3]),
                },
                TestConstraint::Ineq {
                    expr: "4.0 - (3.0 * x[0] + x[1] + 2.0 * x[2] - x[3])".to_string(),
                    fun: |x| 4.0 - (3.0 * x[0] + x[1] + 2.0 * x[2] - x[3]),
                },
                TestConstraint::Ineq {
                    expr: "x[1] + 4.0 * x[2] - 1.5".to_string(),
                    fun: |x| x[1] + 4.0 * x[2] - 1.5,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS78
        TestCase {
            name: "HS78".to_string(),
            objective_expr: "x[0]*x[1]*x[2]*x[3]*x[4]".to_string(),
            objective_fn: |x| x[0]*x[1]*x[2]*x[3]*x[4],
            x0: vec![-2.0, 1.5, 2.0, -1.0, -1.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0]**2 + x[1]**2 + x[2]**2 + x[3]**2 + x[4]**2 - 10.0".to_string(),
                    fun: |x| x[0].powi(2) + x[1].powi(2) + x[2].powi(2) + x[3].powi(2) + x[4].powi(2) - 10.0,
                },
                TestConstraint::Eq {
                    expr: "x[1]*x[2] - 5.0*x[3]*x[4]".to_string(),
                    fun: |x| x[1]*x[2] - 5.0*x[3]*x[4],
                },
                TestConstraint::Eq {
                    expr: "x[0]**3 + x[1]**3 + 1.0".to_string(),
                    fun: |x| x[0].powi(3) + x[1].powi(3) + 1.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS79
        TestCase {
            name: "HS79".to_string(),
            objective_expr: "(x[0]-1.0)**2 + (x[1]-x[2])**2 + (x[3]-x[4])**2".to_string(),
            objective_fn: |x| (x[0]-1.0).powi(2) + (x[1]-x[2]).powi(2) + (x[3]-x[4]).powi(2),
            x0: vec![2.0, 2.0, 2.0, 2.0, 2.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + x[1] + x[2] + x[3] + x[4] - 5.0".to_string(),
                    fun: |x| x[0] + x[1] + x[2] + x[3] + x[4] - 5.0,
                },
                TestConstraint::Eq {
                    expr: "x[0]**2 + x[1]**2 + x[2]**2 + x[3]**2 + x[4]**2 - 10.0".to_string(),
                    fun: |x| x[0].powi(2) + x[1].powi(2) + x[2].powi(2) + x[3].powi(2) + x[4].powi(2) - 10.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS80
        TestCase {
            name: "HS80".to_string(),
            objective_expr: "exp(x[0]*x[1]*x[2]*x[3]*x[4])".to_string(),
            objective_fn: |x| (x[0]*x[1]*x[2]*x[3]*x[4]).exp(),
            x0: vec![-2.0, 2.0, 2.0, -1.0, -1.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0]**2 + x[1]**2 + x[2]**2 + x[3]**2 + x[4]**2 - 10.0".to_string(),
                    fun: |x| x[0].powi(2) + x[1].powi(2) + x[2].powi(2) + x[3].powi(2) + x[4].powi(2) - 10.0,
                },
                TestConstraint::Eq {
                    expr: "x[1]*x[2] - 5.0*x[3]*x[4]".to_string(),
                    fun: |x| x[1]*x[2] - 5.0*x[3]*x[4],
                },
                TestConstraint::Eq {
                    expr: "x[0]**3 + x[1]**3 + 1.0".to_string(),
                    fun: |x| x[0].powi(3) + x[1].powi(3) + 1.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS81
        TestCase {
            name: "HS81".to_string(),
            objective_expr: "exp(x[0]*x[1]*x[2]*x[3]*x[4])".to_string(),
            objective_fn: |x| (x[0]*x[1]*x[2]*x[3]*x[4]).exp(),
            x0: vec![-2.0, 2.0, 2.0, -1.0, -1.0],
            bounds: Some(vec![(-2.3, 2.3), (-2.3, 2.3), (-3.2, 3.2), (-3.2, 3.2), (-3.2, 3.2)]),
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0]**2 + x[1]**2 + x[2]**2 + x[3]**2 + x[4]**2 - 10.0".to_string(),
                    fun: |x| x[0].powi(2) + x[1].powi(2) + x[2].powi(2) + x[3].powi(2) + x[4].powi(2) - 10.0,
                },
                TestConstraint::Eq {
                    expr: "x[1]*x[2] - 5.0*x[3]*x[4]".to_string(),
                    fun: |x| x[1]*x[2] - 5.0*x[3]*x[4],
                },
                TestConstraint::Eq {
                    expr: "x[0]**3 + x[1]**3 + 1.0".to_string(),
                    fun: |x| x[0].powi(3) + x[1].powi(3) + 1.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS82
        TestCase {
            name: "HS82".to_string(),
            objective_expr: "(x[0]-1.0)**2 + (x[0]-x[1])**2 + (x[1]-x[2])**2".to_string(),
            objective_fn: |x| (x[0]-1.0).powi(2) + (x[0]-x[1]).powi(2) + (x[1]-x[2]).powi(2),
            x0: vec![2.0, 2.0, 2.0],
            bounds: Some(vec![(-10.0, 10.0), (-10.0, 10.0), (-10.0, 10.0)]),
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] * (1.0 + x[1]**2) + x[2]**4 - (4.0 + 3.0 * sqrt(2.0))".to_string(),
                    fun: |x| x[0] * (1.0 + x[1].powi(2)) + x[2].powi(4) - (4.0 + 3.0 * 2.0f64.sqrt()),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS83
        TestCase {
            name: "HS83".to_string(),
            objective_expr: "5.3578547 * x[2]**2 + 0.835608 * x[0] * x[4] + 37.293239 * x[0] - 40792.141".to_string(),
            objective_fn: |x| 5.3578547 * x[2].powi(2) + 0.835608 * x[0] * x[4] + 37.293239 * x[0] - 40792.141,
            x0: vec![78.0, 33.0, 27.0, 27.0, 27.0],
            bounds: Some(vec![(78.0, 102.0), (33.0, 45.0), (27.0, 45.0), (27.0, 45.0), (27.0, 45.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "92.0 - (85.334407 + 0.0056858 * x[1] * x[4] + 0.0006262 * x[0] * x[3] - 0.0022053 * x[2] * x[4])".to_string(),
                    fun: |x| 92.0 - (85.334407 + 0.0056858 * x[1] * x[4] + 0.0006262 * x[0] * x[3] - 0.0022053 * x[2] * x[4]),
                },
                TestConstraint::Ineq {
                    expr: "85.334407 + 0.0056858 * x[1] * x[4] + 0.0006262 * x[0] * x[3] - 0.0022053 * x[2] * x[4]".to_string(),
                    fun: |x| 85.334407 + 0.0056858 * x[1] * x[4] + 0.0006262 * x[0] * x[3] - 0.0022053 * x[2] * x[4],
                },
                TestConstraint::Ineq {
                    expr: "20.0 - (80.51249 + 0.0071317 * x[1] * x[4] + 0.0029955 * x[0] * x[1] + 0.0021813 * x[2]**2 - 90.0)".to_string(),
                    fun: |x| 20.0 - (80.51249 + 0.0071317 * x[1] * x[4] + 0.0029955 * x[0] * x[1] + 0.0021813 * x[2].powi(2) - 90.0),
                },
                TestConstraint::Ineq {
                    expr: "12.0 - (9.300961 + 0.0047026 * x[2] * x[4] + 0.0012547 * x[0] * x[2] + 0.0019085 * x[2] * x[3] - 20.0)".to_string(),
                    fun: |x| 12.0 - (9.300961 + 0.0047026 * x[2] * x[4] + 0.0012547 * x[0] * x[2] + 0.0019085 * x[2] * x[3] - 20.0),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS84
        TestCase {
            name: "HS84".to_string(),
            objective_expr: "-(x[0]+x[1]+x[2]+x[3]+x[4])".to_string(),
            objective_fn: |x| -(x[0]+x[1]+x[2]+x[3]+x[4]),
            x0: vec![2.52, 2.0, 37.5, 9.25, 6.8],
            bounds: Some(vec![(0.0, 100.0), (0.0, 100.0), (0.0, 1000.0), (0.0, 1000.0), (0.0, 1000.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "1.0 - (0.0064*x[0] + 0.000133*x[2] + 0.0000049*x[3])".to_string(),
                    fun: |x| 1.0 - (0.0064*x[0] + 0.000133*x[2] + 0.0000049*x[3]),
                },
                TestConstraint::Ineq {
                    expr: "1.0 - (0.0032*x[0] + 0.0019*x[1] + 0.00001*x[2] + 0.0001*x[3])".to_string(),
                    fun: |x| 1.0 - (0.0032*x[0] + 0.0019*x[1] + 0.00001*x[2] + 0.0001*x[3]),
                },
                TestConstraint::Ineq {
                    expr: "1.0 - (0.0045*x[0] + 0.0051*x[1] + 0.00043*x[2] + 0.0001*x[3] + 0.000035*x[4])".to_string(),
                    fun: |x| 1.0 - (0.0045*x[0] + 0.0051*x[1] + 0.00043*x[2] + 0.0001*x[3] + 0.000035*x[4]),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS86
        TestCase {
            name: "HS86".to_string(),
            objective_expr: "x[0]*x[1]*x[2]*x[3]*x[4]".to_string(),
            objective_fn: |x| x[0]*x[1]*x[2]*x[3]*x[4],
            x0: vec![-1.0, -1.0, -1.0, -1.0, -1.0],
            bounds: Some(vec![(-1.0, 1.0), (-1.0, 1.0), (-1.0, 1.0), (-1.0, 1.0), (-1.0, 1.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "1.0 - (x[0]**2 + x[1]**2 + x[2]**2 + x[3]**2 + x[4]**2)".to_string(),
                    fun: |x| 1.0 - (x[0].powi(2) + x[1].powi(2) + x[2].powi(2) + x[3].powi(2) + x[4].powi(2)),
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // HS77
        TestCase {
            name: "HS77".to_string(),
            objective_expr: "(x[0]-1.0)**2 + (x[0]-x[1])**2 + (x[1]-x[2])**2 + (x[2]-x[3])**4 + (x[3]-x[4])**4".to_string(),
            objective_fn: |x| (x[0]-1.0).powi(2) + (x[0]-x[1]).powi(2) + (x[1]-x[2]).powi(2) + (x[2]-x[3]).powi(4) + (x[3]-x[4]).powi(4),
            x0: vec![2.0, 2.0, 2.0, 2.0, 2.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + x[1]**2 + x[2]**3 - (3.0 * sqrt(2.0) + 2.0)".to_string(),
                    fun: |x| x[0] + x[1].powi(2) + x[2].powi(3) - (3.0 * 2.0f64.sqrt() + 2.0),
                },
                TestConstraint::Eq {
                    expr: "x[1] - x[2]**2 + x[3] - (2.0 * sqrt(2.0) - 2.0)".to_string(),
                    fun: |x| x[1] - x[2].powi(2) + x[3] - (2.0 * 2.0f64.sqrt() - 2.0),
                },
                TestConstraint::Eq {
                    expr: "x[0] * x[4] - 2.0".to_string(),
                    fun: |x| x[0] * x[4] - 2.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // 11. Equality constraint: minimize x[0]^2 + x[1]^2, constraint x[0] + x[1] = 2
        TestCase {
            name: "Sphere with Equality Constraint".to_string(),
            objective_expr: "x[0]**2 + x[1]**2".to_string(),
            objective_fn: |x| x[0].powi(2) + x[1].powi(2),
            x0: vec![0.0, 0.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + x[1] - 2".to_string(),
                    fun: |x| x[0] + x[1] - 2.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // 3. Inequality constraint: minimize x[0]^2 + x[1]^2, constraint x[0] + x[1] >= 1 (i.e., x[0] + x[1] - 1 >= 0)
        TestCase {
            name: "Sphere with Inequality Constraint".to_string(),
            objective_expr: "x[0]**2 + x[1]**2".to_string(),
            objective_fn: |x| x[0].powi(2) + x[1].powi(2),
            x0: vec![2.0, 2.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[0] + x[1] - 1".to_string(),
                    fun: |x| x[0] + x[1] - 1.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // 4. Mixed constraints: minimize x[0]**2 + x[1]**2, constraints x[0] + x[1] = 1, x[0] >= 0.5
        TestCase {
            name: "Sphere with Mixed Constraints".to_string(),
            objective_expr: "x[0]**2 + x[1]**2".to_string(),
            objective_fn: |x| x[0].powi(2) + x[1].powi(2),
            x0: vec![1.0, 1.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + x[1] - 1".to_string(),
                    fun: |x| x[0] + x[1] - 1.0,
                },
                TestConstraint::Ineq {
                    expr: "x[0] - 0.5".to_string(),
                    fun: |x| x[0] - 0.5,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // 5. With bound constraints: minimize x[0]**2 + x[1]**2, 1 <= x[0] <= 2, 1 <= x[1] <= 2
        TestCase {
            name: "Sphere with Bounds".to_string(),
            objective_expr: "x[0]**2 + x[1]**2".to_string(),
            objective_fn: |x| x[0].powi(2) + x[1].powi(2),
            x0: vec![1.5, 1.5],
            bounds: Some(vec![(1.0, 2.0), (1.0, 2.0)]),
            constraints: vec![],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // 6. Mixed inequality and bound constraints: minimize x[0]**2 + x[1]**2, constraint x[0] + x[1] >= 1, 0 <= x[0] <= 0.4
        // Expected solution should be x[0]=0.4, x[1]=0.6
        TestCase {
            name: "Mixed Bounds and Inequality".to_string(),
            objective_expr: "x[0]**2 + x[1]**2".to_string(),
            objective_fn: |x| x[0].powi(2) + x[1].powi(2),
            x0: vec![0.2, 0.8],
            bounds: Some(vec![(0.0, 0.4), (0.0, 10.0)]),
            constraints: vec![
                TestConstraint::Ineq {
                    expr: "x[0] + x[1] - 1.0".to_string(),
                    fun: |x| x[0] + x[1] - 1.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        },
        // 7. Objective function contains exp/log: minimize exp(x[0]) + x[1]**2, x[0] + x[1] = 1
        TestCase {
            name: "Exponential Objective with Equality".to_string(),
            objective_expr: "exp(x[0]) + x[1]**2".to_string(),
            objective_fn: |x| x[0].exp() + x[1].powi(2),
            x0: vec![0.0, 0.0],
            bounds: None,
            constraints: vec![
                TestConstraint::Eq {
                    expr: "x[0] + x[1] - 1.0".to_string(),
                    fun: |x| x[0] + x[1] - 1.0,
                }
            ],
            tol: 1e-6,
            maxiter: 100,
            comparison_tol: None,
        }
    ]
}
