use core::marker::PhantomData;

#[derive(Clone, Debug)]
#[repr(C, align(64))]
pub struct DualCacheFF<
    K,
    V,
    P,
    const C2: usize,
    const C1: usize,
    const C0: usize,
    const TC: usize,
    const P4: usize = 0,
    const P5: usize = 0,
    const P6: usize = 0,
> {
    _marker: PhantomData<(K, V, P)>,
}

unsafe impl<
    K,
    V,
    P,
    const C2: usize,
    const C1: usize,
    const C0: usize,
    const TC: usize,
    const P4: usize,
    const P5: usize,
    const P6: usize,
> Send for DualCacheFF<K, V, P, C2, C1, C0, TC, P4, P5, P6>
{
}

unsafe impl<
    K,
    V,
    P,
    const C2: usize,
    const C1: usize,
    const C0: usize,
    const TC: usize,
    const P4: usize,
    const P5: usize,
    const P6: usize,
> Sync for DualCacheFF<K, V, P, C2, C1, C0, TC, P4, P5, P6>
{
}

impl<
    K,
    V,
    P,
    const C2: usize,
    const C1: usize,
    const C0: usize,
    const TC: usize,
    const P4: usize,
    const P5: usize,
    const P6: usize,
> DualCacheFF<K, V, P, C2, C1, C0, TC, P4, P5, P6>
{
    pub fn new<Config>(_config: Config) -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}
