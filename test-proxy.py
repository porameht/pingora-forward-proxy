#!/usr/bin/env python3
"""
Test script for Pingora Multi-IP Proxy
Tests proxy functionality and IP rotation
"""

import requests
import sys
from collections import Counter

def test_proxy(proxy_host, proxy_port, username, password, num_requests=20):
    """Test the proxy and verify IP rotation"""

    print("🧪 Pingora Multi-IP Proxy Test Script")
    print("=" * 50)
    print(f"Proxy: {proxy_host}:{proxy_port}")
    print(f"Requests: {num_requests}")
    print("=" * 50)
    print()

    proxies = {
        'http': f'http://{username}:{password}@{proxy_host}:{proxy_port}',
        'https': f'http://{username}:{password}@{proxy_host}:{proxy_port}'
    }

    # Test endpoints
    test_url = 'https://httpbin.org/ip'

    source_ips = []
    success_count = 0
    failure_count = 0

    print("🚀 Starting tests...\n")

    for i in range(1, num_requests + 1):
        try:
            response = requests.get(test_url, proxies=proxies, timeout=10)

            if response.status_code == 200:
                data = response.json()
                source_ip = data.get('origin', 'Unknown')
                source_ips.append(source_ip)

                print(f"✅ Request {i:2d}/{num_requests}: {source_ip}")
                success_count += 1
            else:
                print(f"❌ Request {i:2d}/{num_requests}: HTTP {response.status_code}")
                failure_count += 1

        except requests.exceptions.ProxyError as e:
            print(f"❌ Request {i:2d}/{num_requests}: Proxy Error - {e}")
            failure_count += 1
        except requests.exceptions.Timeout:
            print(f"❌ Request {i:2d}/{num_requests}: Timeout")
            failure_count += 1
        except Exception as e:
            print(f"❌ Request {i:2d}/{num_requests}: Error - {e}")
            failure_count += 1

    # Statistics
    print("\n" + "=" * 50)
    print("📊 Test Results")
    print("=" * 50)
    print(f"Total Requests:    {num_requests}")
    print(f"Successful:        {success_count} ({success_count/num_requests*100:.1f}%)")
    print(f"Failed:            {failure_count} ({failure_count/num_requests*100:.1f}%)")

    if source_ips:
        ip_counts = Counter(source_ips)
        unique_ips = len(ip_counts)

        print(f"\nUnique IPs:        {unique_ips}")
        print(f"\nIP Distribution:")

        for ip, count in ip_counts.most_common():
            percentage = (count / success_count) * 100
            bar = "█" * int(percentage / 2)
            print(f"  {ip}: {count:2d} requests ({percentage:5.1f}%) {bar}")

        # Check if rotation is working
        print("\n" + "=" * 50)
        if unique_ips > 1:
            print("✅ IP Rotation: WORKING")
            print(f"   Rotating through {unique_ips} different IPs")
        elif unique_ips == 1:
            print("⚠️  IP Rotation: NOT WORKING")
            print("   Only using 1 IP address")
            print("   Check your IP pool configuration")

        # Check distribution evenness
        if unique_ips > 1:
            expected_per_ip = success_count / unique_ips
            max_deviation = max(abs(count - expected_per_ip) for count in ip_counts.values())

            if max_deviation / expected_per_ip < 0.3:
                print("✅ Distribution: EVEN")
            else:
                print("⚠️  Distribution: UNEVEN")
    else:
        print("\n❌ No successful requests - cannot analyze IP rotation")

    print("=" * 50)

    return success_count > 0

def test_authentication(proxy_host, proxy_port):
    """Test authentication requirement"""

    print("\n🔐 Testing Authentication...")
    print("=" * 50)

    # Test without auth
    print("Testing without credentials...")
    try:
        proxies = {
            'http': f'http://{proxy_host}:{proxy_port}',
            'https': f'http://{proxy_host}:{proxy_port}'
        }
        response = requests.get('https://httpbin.org/ip', proxies=proxies, timeout=5)

        if response.status_code == 407:
            print("✅ Authentication required (407 returned)")
            return True
        else:
            print(f"⚠️  Expected 407, got {response.status_code}")
            return False

    except requests.exceptions.ProxyError:
        print("✅ Authentication required (proxy rejected connection)")
        return True
    except Exception as e:
        print(f"❌ Error: {e}")
        return False

if __name__ == "__main__":
    # Configuration
    PROXY_HOST = sys.argv[1] if len(sys.argv) > 1 else "172.105.123.45"
    PROXY_PORT = int(sys.argv[2]) if len(sys.argv) > 2 else 7777
    USERNAME = sys.argv[3] if len(sys.argv) > 3 else "proxy_user"
    PASSWORD = sys.argv[4] if len(sys.argv) > 4 else "proxy_pass"
    NUM_REQUESTS = int(sys.argv[5]) if len(sys.argv) > 5 else 20

    print()
    print("Usage: python3 test-proxy.py [host] [port] [username] [password] [num_requests]")
    print(f"Example: python3 test-proxy.py {PROXY_HOST} {PROXY_PORT} {USERNAME} {PASSWORD} {NUM_REQUESTS}")
    print()

    # Run tests
    try:
        # Test authentication
        test_authentication(PROXY_HOST, PROXY_PORT)

        # Test proxy functionality and IP rotation
        success = test_proxy(PROXY_HOST, PROXY_PORT, USERNAME, PASSWORD, NUM_REQUESTS)

        if success:
            print("\n✅ All tests completed successfully!")
            sys.exit(0)
        else:
            print("\n❌ Tests failed!")
            sys.exit(1)

    except KeyboardInterrupt:
        print("\n\n⚠️  Tests interrupted by user")
        sys.exit(1)
    except Exception as e:
        print(f"\n❌ Unexpected error: {e}")
        sys.exit(1)
