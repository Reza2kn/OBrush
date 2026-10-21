/// Column-major Mat
#[derive(Debug, Clone, Default)]
pub struct Mat {
    pub data: Vec<f64>,
    pub m: usize,
    pub n: usize,
}

impl Mat {
    pub fn new(m: usize, n: usize) -> Self {
        Self {
            data: vec![0.0; m * n],
            m,
            n,
        }
    }

    pub fn from_data(m: usize, n: usize, data: Vec<f64>) -> Self {
        assert_eq!(data.len(), m * n);
        Self { data, m, n }
    }

    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.data[j * self.m + i]
    }

    pub fn set(&mut self, i: usize, j: usize, val: f64) {
        self.data[j * self.m + i] = val;
    }

    pub fn col(&self, j: usize) -> &[f64] {
        let start = j * self.m;
        &self.data[start..start + self.m]
    }

    pub fn col_mut(&mut self, j: usize) -> &mut [f64] {
        let start = j * self.m;
        &mut self.data[start..start + self.m]
    }

    pub fn fill(&mut self, val: f64) {
        for x in self.data.iter_mut() {
            *x = val;
        }
    }

    pub fn identify(&mut self) {
        self.fill(0.0);
        for i in 0..self.m {
            self[(i, i)] = 1.0;
        }
    }

    pub fn as_slice(&self) -> &[f64] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [f64] {
        &mut self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn rows(&self) -> usize {
        self.m
    }

    pub fn cols(&self) -> usize {
        self.n
    }

    pub fn view_mut(&mut self) -> MatView<'_> {
        MatView::new(self.m, self.n, &mut self.data)
    }
}

/// Column-major MatView which doesn't own data
#[derive(Debug)]
pub struct MatView<'a> {
    pub data: &'a mut [f64],
    pub m: usize,
    pub n: usize,
}

impl<'a> MatView<'a> {
    pub fn new(m: usize, n: usize, data: &'a mut [f64]) -> Self {
        assert_eq!(data.len(), m * n);
        Self { data, m, n }
    }

    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.data[j * self.m + i]
    }

    pub fn set(&mut self, i: usize, j: usize, val: f64) {
        self.data[j * self.m + i] = val;
    }

    pub fn col(&self, j: usize) -> &[f64] {
        let start = j * self.m;
        &self.data[start..start + self.m]
    }

    pub fn col_mut(&mut self, j: usize) -> &mut [f64] {
        let start = j * self.m;
        &mut self.data[start..start + self.m]
    }

    pub fn rows(&self) -> usize {
        self.m
    }

    pub fn cols(&self) -> usize {
        self.n
    }
}

impl<'a> std::ops::Index<usize> for MatView<'a> {
    type Output = f64;
    fn index(&self, index: usize) -> &f64 {
        &self.data[index]
    }
}

impl<'a> std::ops::IndexMut<usize> for MatView<'a> {
    fn index_mut(&mut self, index: usize) -> &mut f64 {
        &mut self.data[index]
    }
}

impl<'a> std::ops::Index<(usize, usize)> for MatView<'a> {
    type Output = f64;
    fn index(&self, index: (usize, usize)) -> &f64 {
        &self.data[index.1 * self.m + index.0]
    }
}

impl<'a> std::ops::IndexMut<(usize, usize)> for MatView<'a> {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut f64 {
        &mut self.data[index.1 * self.m + index.0]
    }
}

impl std::ops::Index<usize> for Mat {
    type Output = f64;
    fn index(&self, index: usize) -> &f64 {
        &self.data[index]
    }
}

impl std::ops::IndexMut<usize> for Mat {
    fn index_mut(&mut self, index: usize) -> &mut f64 {
        &mut self.data[index]
    }
}

impl std::ops::Index<(usize, usize)> for Mat {
    type Output = f64;
    fn index(&self, index: (usize, usize)) -> &f64 {
        &self.data[index.1 * self.m + index.0]
    }
}

impl std::ops::IndexMut<(usize, usize)> for Mat {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut f64 {
        &mut self.data[index.1 * self.m + index.0]
    }
}

impl std::fmt::Display for Mat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..self.m {
            for j in 0..self.n {
                write!(f, "{:10.4} ", self.get(i, j))?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
