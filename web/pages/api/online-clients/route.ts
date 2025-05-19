import {NextResponse} from 'next/server';

import {online_clients} from '../websocket';

export async function GET() {
  // Convert the online_clients Map to an array of BoardInfo
  const clients = Array.from(online_clients.values());
  return NextResponse.json({clients});
}
