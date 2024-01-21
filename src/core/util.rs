pub fn extract_arguments(args: &str) -> Vec<(&str, Vec<&str>)> {
    let vec_args: Vec<&str> = args.split("--").collect();

    let arguments: Vec<(&str, Vec<&str>)> = vec_args
        .iter()
        .map(|&arg| {
            let split: Vec<&str> = arg.split(' ').collect();
            (split[0], split[1..].to_vec())
        })
        .filter(|(name, args)| !name.is_empty() && !args.is_empty())
        .collect();

    arguments
}
