/**
 * Wisp has a handful of screens, so it uses a plain discriminated union
 * instead of a router dependency. `GameRoute` already carries the Legendary
 * app name that the detail page (sprint 2) will need.
 */

export type Route =
  | { page: 'library' }
  | { page: 'downloads' }
  | { page: 'settings' }
  | { page: 'game'; appName: string };

export type RouteName = Route['page'];

export const DEFAULT_ROUTE: Route = { page: 'library' };
