use std::net::IpAddr;

/// 从接口列表里选出最适合展示给手机扫码的局域网 IP。
///
/// 规则：
/// - 只要 IPv4、非回环
/// - 排除虚拟/专用接口：utun（VPN）、awdl/llw（AirDrop 蓝牙）、tap/tun、
///   anpi（Apple 网桥）、gif/stf（隧道）、lo
/// - 热点/共享接口最高优先：macOS 互联网共享的 bridge*、Windows 移动热点（名称含 *）
/// - 其次 192.168/16 > 10/8 > 172.16-31/12 > 其他非回环
pub fn best_lan_ip(interfaces: &[(String, IpAddr)]) -> Option<IpAddr> {
    let mut candidates: Vec<(IpAddr, u8)> = interfaces
        .iter()
        .filter(|(name, ip)| ip.is_ipv4() && !ip.is_loopback())
        .filter(|(name, _)| !is_virtual_iface(name))
        .map(|(name, ip)| (*ip, priority(name, ip)))
        .collect();
    candidates.sort_by_key(|(_, p)| *p);
    candidates.first().map(|(ip, _)| *ip)
}

/// 从本机真实接口中检测（生产路径）
pub fn detect_lan_ip() -> Option<IpAddr> {
    let ifaces = if_addrs::get_if_addrs().ok()?;
    let list: Vec<(String, IpAddr)> = ifaces.iter().map(|i| (i.name.clone(), i.ip())).collect();
    best_lan_ip(&list)
}

fn is_virtual_iface(name: &str) -> bool {
    let n = name.to_lowercase();
    // 注意：不排除 bridge* —— macOS 互联网共享（电脑开热点）的接口正是 bridge100
    // 不按 "lo" 前缀排除：回环 IP(127.x) 已由 is_loopback() 过滤，而 "local..." 会误伤
    n.starts_with("utun")
        || n.starts_with("awdl")
        || n.starts_with("llw")
        || n.starts_with("tap")
        || n.starts_with("tun")
        || n.starts_with("anpi")
        || n.starts_with("gif")
        || n.starts_with("stf")
        || n.starts_with("pdp_ip")
}

/// 优先级：数值越小越优先。
/// 热点/共享接口（bridge*、Windows 移动热点名称含 *）排最前，
/// 因为「电脑开热点让手机连」正是 AirBox 的核心场景，此时必须用热点的 IP。
fn priority(name: &str, ip: &IpAddr) -> u8 {
    let n = name.to_lowercase();
    if n.starts_with("bridge") || n.contains('*') {
        return 0;
    }
    if let IpAddr::V4(v4) = ip {
        let o = v4.octets();
        if o[0] == 192 && o[1] == 168 {
            1
        } else if o[0] == 10 {
            2
        } else if o[0] == 172 && (16..=31).contains(&o[1]) {
            3
        } else {
            4
        }
    } else {
        5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn v4(a: u8, b: u8, c: u8, d: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(a, b, c, d))
    }

    fn iface(name: &str, ip: IpAddr) -> (String, IpAddr) {
        (name.to_string(), ip)
    }

    #[test]
    fn prefers_192_168_over_10_and_vpn() {
        let list = vec![
            iface("utun4", v4(192, 168, 1, 5)), // VPN 虚拟口必须被排除
            iface("awdl0", v4(169, 254, 12, 34)),
            iface("en0", v4(10, 0, 0, 2)),
            iface("en1", v4(192, 168, 31, 88)),
        ];
        assert_eq!(best_lan_ip(&list), Some(v4(192, 168, 31, 88)));
    }

    #[test]
    fn falls_back_to_10_when_no_192() {
        let list = vec![
            iface("en0", v4(10, 1, 2, 3)),
            iface("utun0", v4(10, 9, 9, 9)),
        ];
        assert_eq!(best_lan_ip(&list), Some(v4(10, 1, 2, 3)));
    }

    #[test]
    fn accepts_172_private_range() {
        let list = vec![iface("en0", v4(172, 20, 0, 1))];
        assert_eq!(best_lan_ip(&list), Some(v4(172, 20, 0, 1)));
    }

    #[test]
    fn ignores_loopback_and_ipv6() {
        let list = vec![
            iface("lo0", v4(127, 0, 0, 1)),
            iface("en0", IpAddr::V6("fe80::1".parse().unwrap())),
        ];
        assert_eq!(best_lan_ip(&list), None);
    }

    #[test]
    fn mac_internet_sharing_bridge_wins() {
        // macOS 开「互联网共享」热点：bridge100 是热点接口，必须优先于家里 Wi-Fi
        let list = vec![
            iface("en0", v4(192, 168, 1, 5)),   // 家里 Wi-Fi
            iface("bridge100", v4(192, 168, 2, 1)), // 热点
            iface("utun4", v4(192, 168, 1, 9)),  // VPN
        ];
        assert_eq!(best_lan_ip(&list), Some(v4(192, 168, 2, 1)));
    }

    #[test]
    fn windows_mobile_hotspot_wins() {
        // Windows 移动热点：适配器名称带 *，IP 192.168.137.1
        let list = vec![
            iface("Ethernet", v4(192, 168, 1, 5)),
            iface("Local Area Connection* 12", v4(192, 168, 137, 1)),
        ];
        assert_eq!(best_lan_ip(&list), Some(v4(192, 168, 137, 1)));
    }

    #[test]
    fn empty_list_returns_none() {
        assert_eq!(best_lan_ip(&[]), None);
    }
}
