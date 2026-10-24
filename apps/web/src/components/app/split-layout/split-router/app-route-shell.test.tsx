import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { createSignal } from 'solid-js';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';

const [authenticated, setAuthenticated] = createSignal<boolean | undefined>();
const navigate = vi.fn();
const mountView = vi.fn();

vi.mock('@core/auth', () => ({ useIsAuthenticated: () => authenticated }));
vi.mock('@solidjs/router', () => ({ useNavigate: () => navigate }));
vi.mock('@core/component/LoadingBlock', () => ({
  LoadingBlock: () => <div>Loading</div>,
}));
vi.mock('@app/lib/analytics/analytics-context', () => ({
  useAnalytics: () => ({ pageView: vi.fn(), track: vi.fn() }),
}));
vi.mock('@app/lib/split-router', () => ({
  isRecord: () => false,
  useSplitRouter: () => ({}),
}));
vi.mock('../layoutUtils', () => ({ useSplitPanelOrThrow: () => ({}) }));

import { withAuth } from './app-route-shell';

const Gated = withAuth(
  () => {
    mountView();
    return <div>Account view</div>;
  },
  () => <div>Confirming session</div>
);

beforeEach(() => setAuthenticated(undefined));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

it('offers sign-in instead of an indefinite loader once sign-out is confirmed', () => {
  setAuthenticated(false);
  render(() => <Gated />);

  expect(screen.getByText('Sign in to view this page')).toBeTruthy();
  expect(screen.queryByText('Loading')).toBeNull();
  expect(screen.queryByText('Confirming session')).toBeNull();
  expect(mountView).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Login' }));
  expect(navigate).toHaveBeenCalledWith('/login');
});

it('shows the pending state, not sign-in, while identity is unresolved', () => {
  render(() => <Gated />);

  expect(screen.getByText('Confirming session')).toBeTruthy();
  expect(screen.queryByText('Sign in to view this page')).toBeNull();
  setAuthenticated(false);
  expect(screen.getByText('Sign in to view this page')).toBeTruthy();
  setAuthenticated(true);
  expect(screen.getByText('Account view')).toBeTruthy();
  expect(screen.queryByText('Sign in to view this page')).toBeNull();
});
