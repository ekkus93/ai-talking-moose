include!("core.rs");

#[cfg(test)]
mod tests {
    include!("tests.rs");
}

#[cfg(test)]
mod provider_boundary_tests {
    include!("provider_boundary_tests.rs");
}
