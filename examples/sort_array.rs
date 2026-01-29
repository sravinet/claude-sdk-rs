fn sort_array<T: Ord + Clone>(mut arr: Vec<T>) -> Vec<T> {
    arr.sort();
    arr
}

fn sort_array_in_place<T: Ord>(arr: &mut [T]) {
    arr.sort();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_array() {
        let input = vec![3, 1, 4, 1, 5, 9, 2, 6];
        let expected = vec![1, 1, 2, 3, 4, 5, 6, 9];
        assert_eq!(sort_array(input), expected);
    }

    #[test]
    fn test_sort_array_in_place() {
        let mut input = [3, 1, 4, 1, 5, 9, 2, 6];
        let expected = [1, 1, 2, 3, 4, 5, 6, 9];
        sort_array_in_place(&mut input);
        assert_eq!(input, expected);
    }

    #[test]
    fn test_sort_strings() {
        let input = vec!["banana", "apple", "cherry"];
        let expected = vec!["apple", "banana", "cherry"];
        assert_eq!(sort_array(input), expected);
    }
}