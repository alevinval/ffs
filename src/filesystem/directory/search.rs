pub fn binary_search<'slice, T, K>(
    input: &'slice [T],
    value: &K,
    get_key: impl Fn(&T) -> &K,
) -> Option<&'slice T>
where
    K: Ord + ?Sized,
{
    binary_search_index(input, value, get_key).map(|idx| &input[idx])
}

pub fn binary_search_mut<'slice, T, K>(
    input: &'slice mut [T],
    value: &K,
    get_key: impl Fn(&T) -> &K,
) -> Option<&'slice mut T>
where
    K: Ord + ?Sized,
{
    binary_search_index(input, value, get_key).map(|idx| &mut input[idx])
}

fn binary_search_index<T, K>(list: &[T], value: &K, get_key: impl Fn(&T) -> &K) -> Option<usize>
where
    K: Ord + ?Sized,
{
    let mut low = 0;
    let mut high = list.len();
    while low < high {
        let mid = (low + high) / 2;
        match get_key(&list[mid]).cmp(value) {
            core::cmp::Ordering::Less => low = mid + 1,
            core::cmp::Ordering::Equal => return Some(mid),
            core::cmp::Ordering::Greater => high = mid,
        }
    }
    None
}
