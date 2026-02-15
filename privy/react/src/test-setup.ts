import { vi } from 'vitest';

// Stub Privy app ID for tests (avoids undefined errors in components)
vi.stubEnv('VITE_PRIVY_APP_ID', 'test-app-id');

// NOTE: VITE_HELIUS_RPC_URL is intentionally NOT stubbed here.
// - Unit tests mock createRpc() entirely, so the URL is irrelevant.
// - Integration tests check for the real URL and skip when it's not set.
