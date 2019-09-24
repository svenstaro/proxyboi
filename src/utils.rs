use std::fmt;

#[derive(Debug, Clone, Default)]
pub struct ForwardedHeader {
    pub by: String,
    pub for_: Vec<String>,
    pub host: String,
    pub proto: String,
}

/// We want to append the current host to the forwarded for list.
/// In order to do this, we have to parse the possibly existing Forwarded and X-Forwarded-For
/// headers and append the current host value those.
/// This is done independently for Forwarded and X-Forwarded-For because even though it would be
/// very odd for them to have different values, it's certainly possible and not technically
/// invalid.
/// The Forwarded header is a bit nasty to parse. It can look like this:
/// Forwarded: by=<by>;for=<foo>;host=<host>;proto=<http|https>
/// but also like this
/// Forwarded: for=<foo>
/// but also this
/// Forwarded: for=<foo>, for=<bar>
/// also finally also this
/// Forwarded: by=<by>;for=<foo>, for=<bar>;host=<host>
impl ForwardedHeader {
    pub fn from_info(
        peer: &str,
        interface: &str,
        forwarded: &str,
        host: &str,
        proto: &str,
    ) -> Self {
        // Try to find a `for=` in the value of this `Forwarded` header.
        // If we do find one, we'll have to figure out how many values there are already.
        // If there is no `for=` just yet we can just plop in our own value and be done with it.
        let for_start = forwarded.find("for=");
        let mut fors = vec![];
        if let Some(for_start) = for_start {
            // Try to find a `:` which ends this `for` subfield.
            // If there is none, this field is the last one in the header.
            let forwarded_for = if let Some(for_end) = forwarded[for_start..].find(';') {
                &forwarded[for_start..for_start + for_end]
            } else {
                &forwarded[for_start..forwarded.len()]
            };

            // Now, let's extract all the `for=` from this. They're all separated by `,`s.
            for for_ in forwarded_for.split(',') {
                fors.push(for_.to_string().replace("for=", ""));
            }
        }
        fors.push(peer.to_string());

        ForwardedHeader {
            by: interface.to_string(),
            for_: fors,
            host: host.to_string(),
            proto: proto.to_string(),
        }
    }
}

impl fmt::Display for ForwardedHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "by={by};{for_};host={host};proto={proto}",
            by = self.by,
            for_ = self
                .for_
                .iter()
                .map(|x| format!("for={}", x))
                .collect::<Vec<_>>()
                .join(", "),
            host = self.host,
            proto = self.proto
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_unknown_peer_and_empty_for() {
        let result =
            ForwardedHeader::from_info("unknown", "0.0.0.0", "", "unknown", "http").to_string();
        let expected = "by=0.0.0.0;for=unknown;host=unknown;proto=http";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_known_peer_and_host() {
        let result =
            ForwardedHeader::from_info("192.168.0.100", "0.0.0.0", "", "localhost:8080", "http")
                .to_string();
        let expected = "by=0.0.0.0;for=192.168.0.100;host=localhost:8080;proto=http";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_known_peer_and_host_with_previous_for() {
        let result = ForwardedHeader::from_info(
            "192.168.0.100",
            "0.0.0.0",
            "for=192.168.0.99",
            "localhost:8080",
            "http",
        )
        .to_string();
        let expected =
            "by=0.0.0.0;for=192.168.0.99, for=192.168.0.100;host=localhost:8080;proto=http";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_known_peer_and_host_with_multiple_previous_for() {
        let result = ForwardedHeader::from_info(
            "192.168.0.100",
            "0.0.0.0",
            "for=192.168.0.97,for=192.168.0.98,for=192.168.0.99",
            "localhost:8080",
            "http",
        )
        .to_string();
        let expected =
            "by=0.0.0.0;for=192.168.0.97, for=192.168.0.98, for=192.168.0.99, for=192.168.0.100;host=localhost:8080;proto=http";
        assert_eq!(result, expected);
    }
}
