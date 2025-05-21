'use server';

export type BoardInfo = {
  device_id: string; datetime: string; schedule_version: number;
  running_program: string | null;
  running_zones: {zone_ids: string[]; duration_seconds: number;} | null;
  zones: string[];
  log: string | null;
};

export type ZoneInfo = {
  id: string; name: string;
};

export type BoardDetails = {
  device_id: string; name: string; datetime: string; schedule_version: number;
  running_program: string | null;
  running_zones: ZoneAction | null;
  zones: ZoneInfo[];
};

// If ZoneAction is not defined elsewhere, you may need to define it as well
// Example placeholder:
export type ZoneAction = {
  zone_ids: string[]; duration_seconds: number;
};

export async function addBoard(device_id: string) {
  await fetch(`http://server:3400/boards/add/${device_id}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
  });
}

export async function removeBoard(device_id: string) {
  await fetch(`http://server:3400/boards/remove/${device_id}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
  });
}

export async function updateBoard(
    device_id: string, name: string, zones: {id: string; name: string}[]) {
  await fetch(`http://server:3400/boards/update/${device_id}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({name, zones}),
  });
}

// Szerver komponens, szerver oldali fetch-csel
export async function getOnlineClients(): Promise<BoardInfo[]> {
  const res = await fetch('http://server:3400/online_devices', {
    cache: 'no-store',
  });
  if (!res.ok) return [];
  const data = await res.json();
  console.log('Fetched data:', data);
  // Ensure the return type matches BoardInfo[]
  return Array.isArray(data) ? data as BoardInfo[] :
                               (data.clients as BoardInfo[]) || [];
}

export async function getRegisteredBoards(): Promise<BoardDetails[]> {
  const res = await fetch('http://server:3400/devices', {
    cache: 'no-store',
  });
  if (!res.ok) return [];
  const data = await res.json();
  return Array.isArray(data) ? data : [];
}

export async function getZonesAll(): Promise<ZoneInfo[]> {
  const boards = await getRegisteredBoards();
  const zones = boards.flatMap((board) => board.zones);
  return zones;
}