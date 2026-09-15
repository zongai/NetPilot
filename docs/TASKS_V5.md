# V5 Tasks

| ID | Stage | Task | Dependency | Target |
|---|---|---|---|---|
| NP-145 | F0 | Real Core runtime | V4 + S11 baseline | `apps/core/src/main.rs` |
| NP-146 | F0 | Desktop Core launcher | NP-145 | `apps/desktop/MainWindow.xaml.cs` |
| NP-147 | F0 | Core health state | NP-146 | `crates/core/src/health.rs` |
| NP-148 | F0 | Graceful shutdown | NP-147 | `crates/core/src/shutdown.rs` |
| NP-149 | F0 | Desktop startup state | NP-148 | `apps/desktop/MainWindow.xaml.cs` |
| NP-150 | F0 | Single Core instance | NP-149 | `crates/core/src/instance.rs` |
| NP-151 | F0 | Runtime data directories | NP-150 | `crates/config/src/paths.rs` |
| NP-152 | F0 | Structured logging | NP-151 | `crates/core/src/logging.rs` |
| NP-153 | F0 | Core error surface | NP-152 | `apps/desktop/MainWindow.xaml.cs` |
| NP-154 | F0 | Build artifact layout | NP-153 | `scripts/build.ps1` |
| NP-155 | F0 | Smoke runner | NP-154 | `scripts/smoke.ps1` |
| NP-156 | F0 | Bootstrap E2E | NP-155 | `tests/bootstrap/` |
| NP-157 | F1 | IPC server | NP-156 | `crates/ipc/src/server.rs` |
| NP-158 | F1 | IPC client | NP-157 | `apps/desktop/` |
| NP-159 | F1 | Protocol envelope | NP-158 | `crates/ipc/src/protocol.rs` |
| NP-160 | F1 | Request correlation | NP-159 | `crates/ipc/src/protocol.rs` |
| NP-161 | F1 | Timeout and cancellation | NP-160 | `crates/ipc/src/client.rs` |
| NP-162 | F1 | IPC error taxonomy | NP-161 | `crates/ipc/src/error.rs` |
| NP-163 | F1 | Handshake | NP-162 | `crates/ipc/src/handshake.rs` |
| NP-164 | F1 | Event subscription | NP-163 | `crates/ipc/src/events.rs` |
| NP-165 | F1 | IPC security boundary | NP-164 | `crates/ipc/src/security.rs` |
| NP-166 | F1 | Command dispatcher | NP-165 | `crates/ipc/src/dispatcher.rs` |
| NP-167 | F1 | IPC integration tests | NP-166 | `crates/ipc/tests/` |
| NP-168 | F1 | Desktop IPC service | NP-167 | `apps/desktop/` |
| NP-169 | F2 | ProxyProfile model | NP-168 | `crates/proxy/src/profile.rs` |
| NP-170 | F2 | Config repository | NP-169 | `crates/config/src/repository.rs` |
| NP-171 | F2 | Proxy registry | NP-170 | `crates/proxy/src/registry.rs` |
| NP-172 | F2 | Proxy group model | NP-171 | `crates/proxy/src/group.rs` |
| NP-173 | F2 | Active proxy runtime | NP-172 | `crates/proxy/src/runtime.rs` |
| NP-174 | F2 | SOCKS5 outbound | NP-173 | `crates/protocols/common/` |
| NP-175 | F2 | HTTP CONNECT outbound | NP-174 | `crates/protocols/common/` |
| NP-176 | F2 | TCP dial policy | NP-175 | `crates/routing/src/dial.rs` |
| NP-177 | F2 | Proxy health check | NP-176 | `crates/proxy/src/health.rs` |
| NP-178 | F2 | Proxy IPC commands | NP-177 | `crates/ipc/src/commands.rs` |
| NP-179 | F2 | Proxy runtime integration | NP-178 | `crates/core/src/runtime.rs` |
| NP-180 | F2 | Proxy E2E | NP-179 | `tests/proxy_e2e/` |
| NP-181 | F3 | Subscription manager | NP-180 | `crates/subscription/src/manager.rs` |
| NP-182 | F3 | HTTP fetcher | NP-181 | `crates/subscription/src/fetch.rs` |
| NP-183 | F3 | Conditional GET | NP-182 | `crates/subscription/src/http_cache.rs` |
| NP-184 | F3 | Persistent cache | NP-183 | `crates/subscription/src/cache.rs` |
| NP-185 | F3 | Base64 decoder | NP-184 | `crates/subscription/src/decode.rs` |
| NP-186 | F3 | URI parser | NP-185 | `crates/subscription/src/parsers/uri.rs` |
| NP-187 | F3 | Clash/Mihomo parser | NP-186 | `crates/subscription/src/parsers/clash.rs` |
| NP-188 | F3 | Sing-box parser | NP-187 | `crates/subscription/src/parsers/singbox.rs` |
| NP-189 | F3 | Normalizer/deduper | NP-188 | `crates/subscription/src/normalize.rs` |
| NP-190 | F3 | Subscription metadata | NP-189 | `crates/subscription/src/metadata.rs` |
| NP-191 | F3 | Subscription IPC | NP-190 | `crates/ipc/src/subscription.rs` |
| NP-192 | F3 | Subscription E2E | NP-191 | `tests/subscription_e2e/` |
| NP-193 | F4 | Rule model | NP-192 | `crates/rules/src/model.rs` |
| NP-194 | F4 | Rule parser | NP-193 | `crates/rules/src/parser.rs` |
| NP-195 | F4 | Domain matcher | NP-194 | `crates/rules/src/domain.rs` |
| NP-196 | F4 | CIDR matcher | NP-195 | `crates/rules/src/ip.rs` |
| NP-197 | F4 | Process matcher | NP-196 | `crates/process/src/matcher.rs` |
| NP-198 | F4 | Rule engine | NP-197 | `crates/routing/src/engine.rs` |
| NP-199 | F4 | Route decision IPC | NP-198 | `crates/ipc/src/routing.rs` |
| NP-200 | F4 | Routing integration | NP-199 | `crates/core/src/runtime.rs` |
| NP-201 | F4 | Rule explanation | NP-200 | `crates/rules/src/explain.rs` |
| NP-202 | F4 | Rules UI | NP-201 | `apps/desktop/` |
| NP-203 | F4 | Rules integration tests | NP-202 | `crates/rules/tests/` |
| NP-204 | F4 | Routing E2E | NP-203 | `tests/routing_e2e/` |
| NP-205 | F5 | Connection model | NP-204 | `crates/core/src/connection.rs` |
| NP-206 | F5 | Connection registry | NP-205 | `crates/core/src/connection_registry.rs` |
| NP-207 | F5 | Connection events | NP-206 | `crates/ipc/src/events.rs` |
| NP-208 | F5 | Connection query IPC | NP-207 | `crates/ipc/src/commands.rs` |
| NP-209 | F5 | Log store | NP-208 | `crates/core/src/log_store.rs` |
| NP-210 | F5 | Log redaction tests | NP-209 | `crates/core/tests/redaction.rs` |
| NP-211 | F5 | Diagnostics commands | NP-210 | `crates/diagnostics/src/commands.rs` |
| NP-212 | F5 | Diagnostics UI | NP-211 | `apps/desktop/` |
| NP-213 | F5 | Connections UI | NP-212 | `apps/desktop/` |
| NP-214 | F5 | Logs UI | NP-213 | `apps/desktop/` |
| NP-215 | F5 | Inspector UI | NP-214 | `apps/desktop/` |
| NP-216 | F5 | Diagnostics E2E | NP-215 | `tests/diagnostics_e2e/` |
| NP-217 | F6 | Navigation state | NP-216 | `apps/desktop/MainWindow.xaml.cs` |
| NP-218 | F6 | Home dashboard | NP-217 | `apps/desktop/` |
| NP-219 | F6 | Proxies page | NP-218 | `apps/desktop/` |
| NP-220 | F6 | Proxy connect UX | NP-219 | `apps/desktop/` |
| NP-221 | F6 | Subscriptions page | NP-220 | `apps/desktop/` |
| NP-222 | F6 | Subscription import UX | NP-221 | `apps/desktop/` |
| NP-223 | F6 | Rules page | NP-222 | `apps/desktop/` |
| NP-224 | F6 | Connections page | NP-223 | `apps/desktop/` |
| NP-225 | F6 | Logs page | NP-224 | `apps/desktop/` |
| NP-226 | F6 | Settings page | NP-225 | `apps/desktop/` |
| NP-227 | F6 | Global error notifications | NP-226 | `apps/desktop/` |
| NP-228 | F6 | UI smoke E2E | NP-227 | `tests/ui_smoke/` |
| NP-229 | F7 | DNS runtime | NP-228 | `crates/dns/src/runtime.rs` |
| NP-230 | F7 | DNS IPC/UI | NP-229 | `crates/ipc/src/dns.rs` |
| NP-231 | F7 | TUN abstraction | NP-230 | `crates/tun/src/lib.rs` |
| NP-232 | F7 | Windows Wintun adapter | NP-231 | `crates/tun/src/windows.rs` |
| NP-233 | F7 | TUN routing integration | NP-232 | `crates/core/src/runtime.rs` |
| NP-234 | F7 | Shadowsocks adapter | NP-233 | `crates/protocols/shadowsocks/` |
| NP-235 | F7 | VMess/VLESS/Trojan adapters | NP-234 | `crates/protocols/` |
| NP-236 | F7 | REALITY capability | NP-235 | `crates/transports/reality/` |
| NP-237 | F7 | Protocol capability matrix | NP-236 | `crates/protocols/common/` |
| NP-238 | F7 | Windows E2E gate | NP-237 | `tests/windows_e2e/` |
| NP-239 | F7 | Installer dependency validation | NP-238 | `scripts/validate-install.ps1` |
| NP-240 | F7 | Functional acceptance harness | NP-239 | `tests/acceptance/` |