'use server';

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