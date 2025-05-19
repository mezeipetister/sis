'use client'

import { useEffect, useState } from 'react';
import { v4 as uuidv4 } from 'uuid';

// Define the type for device status
interface DeviceStatus {
  deviceId: string;
  status: string;
  datetime: string;
  programId: string;
}

interface Program {
  id: string;
  name: string;
  weekdays: number[];
  startTime: string;
  zones: ZoneAction[];
  version: number;
}

interface ZoneAction {
  deviceId: string;
  relay_index: number;
  duration_seconds: number;
}

export default function Home() {
  const [deviceStatuses, setDeviceStatuses] = useState<DeviceStatus[]>([]);
  const [programs, setPrograms] = useState<Program[]>([]);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [newProgram, setNewProgram] = useState<Program>({
    id: '',
    name: '',
    weekdays: [],
    startTime: '',
    zones: [],
    version: 1,
  });

  useEffect(() => {
    const fetchStatuses = async () => {
      try {
        const response = await fetch('/api/status');
        const data = await response.json();
        setDeviceStatuses(data);
      } catch (error) {
        console.error('Error fetching device statuses:', error);
      }
    };

    const fetchPrograms = async () => {
      try {
        const response = await fetch('/api/programs');
        const data = await response.json();
        setPrograms(data);
      } catch (error) {
        console.error('Error fetching programs:', error);
      }
    };

    fetchStatuses();
    fetchPrograms();
  }, []);

  const createProgram = async () => {
    const newProgram: Program = {
      id: uuidv4(),
      name: 'New Program',
      weekdays: [],
      startTime: '',
      zones: [],
      version: 1,
    };

    try {
      const response = await fetch('/api/programs', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(newProgram),
      });
      if (response.ok) {
        setPrograms((prev) => [...prev, newProgram]);
      }
    } catch (error) {
      console.error('Error creating program:', error);
    }
  };

  const deleteProgram = async (id: string) => {
    try {
      const response = await fetch(`/api/programs/${id}`, {
        method: 'DELETE',
      });
      if (response.ok) {
        setPrograms((prev) => prev.filter((program) => program.id !== id));
      }
    } catch (error) {
      console.error('Error deleting program:', error);
    }
  };

  const updateProgram = async (id: string) => {
    const updatedProgram = programs.find((program) => program.id === id);
    if (!updatedProgram) return;

    updatedProgram.version += 1;

    try {
      const response = await fetch(`/api/programs/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(updatedProgram),
      });
      if (response.ok) {
        setPrograms((prev) =>
          prev.map((program) => (program.id === id ? updatedProgram : program))
        );
      }
    } catch (error) {
      console.error('Error updating program:', error);
    }
  };

  const handleModalOpen = () => {
    setIsModalOpen(true);
  };

  const handleModalClose = () => {
    setIsModalOpen(false);
    setNewProgram({
      id: '',
      name: '',
      weekdays: [],
      startTime: '',
      zones: [],
      version: 1,
    });
  };

  const handleSaveProgram = async () => {
    try {
      if (newProgram.id) {
        // Update existing program
        const response = await fetch(`/api/programs/${newProgram.id}`, {
          method: 'PUT',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(newProgram),
        });
        if (response.ok) {
          const updatedProgram = await response.json();
          setPrograms((prev) =>
            prev.map((program) =>
              program.id === newProgram.id ? updatedProgram : program
            )
          );
        }
      } else {
        // Create new program
        const response = await fetch('/api/programs', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ ...newProgram, id: uuidv4() }),
        });
        if (response.ok) {
          const createdProgram = await response.json();
          setPrograms((prev) => [...prev, createdProgram]);
        }
      }
      handleModalClose();
    } catch (error) {
      console.error('Error saving program:', error);
    }
  };

  return (
    <div className="grid grid-rows-[20px_1fr_20px] items-center justify-items-center min-h-screen p-8 pb-20 gap-16 sm:p-20 font-[family-name:var(--font-geist-sans)]">
      <main className="flex flex-col gap-[32px] row-start-2 items-center sm:items-start">

        <h1 className="text-2xl font-bold">Device Statuses</h1>
        <ul className="list-disc">
          {deviceStatuses.map((status) => (
            <li key={status.deviceId} className="mb-2">
              <strong>Device ID:</strong> {status.deviceId} <br />
              <strong>Status:</strong> {status.status} <br />
              <strong>Last Updated:</strong> {status.datetime} <br />
              <strong>Program ID:</strong> {status.programId}
            </li>
          ))}
        </ul>

        <h1 className="text-2xl font-bold">Öntözési terv</h1>
        <button onClick={handleModalOpen} className="px-4 py-2 border-2 border-blue-500 bg-white text-blue-500 rounded hover:bg-blue-100 transition">
          Program hozzáadása
        </button>
        <ul>
          {programs.map((program) => (
            <li key={program.id} className="mb-4">
              <strong>Név:</strong> {program.name} <br />
              <strong>Hét napjai:</strong> {program.weekdays.map(day => {
                const daysInHungarian: Record<number, string> = {
                  1: 'Hétfő',
                  2: 'Kedd',
                  3: 'Szerda',
                  4: 'Csütörtök',
                  5: 'Péntek',
                  6: 'Szombat',
                  7: 'Vasárnap',
                };
                return daysInHungarian[day];
              }).join(', ')} <br />
              <strong>Kezdés:</strong> {program.startTime} <br />
              <strong>Verzió:</strong> {program.version} <br />
              <button
                onClick={() => {
                  setNewProgram(program);
                  setIsModalOpen(true);
                }}
                className="mr-2 px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition"
              >
                Szerkesztés
              </button>
              <button
                onClick={() => {
                  if (confirm('Biztos vagy benne?')) {
                    deleteProgram(program.id);
                  }
                }}
                className="px-4 py-2 bg-red-500 text-white rounded hover:bg-red-600 transition"
              >
                Törlés
              </button>
            </li>
          ))}
        </ul>
      </main>

      {isModalOpen && (
        <div className="modal flex items-center justify-center fixed inset-0 bg-black bg-opacity-50">
          <div className="modal-content p-6 bg-white rounded shadow-md">
            <h2 className="text-xl font-bold mb-4">Új öntözési program</h2>
            <form className="space-y-4">
              <label className="block">
                <span className="text-gray-700">Neve:</span>
                <input
                  type="text"
                  className="mt-1 block w-full rounded border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500"
                  value={newProgram.name}
                  onChange={(e) => setNewProgram({ ...newProgram, name: e.target.value })}
                />
              </label>
              <label className="block">
                <span className="text-gray-700">Kezdés időpontja:</span>
                <input
                  type="time"
                  className="mt-1 block w-full rounded border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500"
                  value={newProgram.startTime}
                  onChange={(e) => setNewProgram({ ...newProgram, startTime: e.target.value })}
                />
              </label>
              <label className="block">
                <span className="text-gray-700">Hét napjai:</span>
                <div className="mt-2 space-y-2">
                  {[
                    { label: 'Hétfő', value: 1 },
                    { label: 'Kedd', value: 2 },
                    { label: 'Szerda', value: 3 },
                    { label: 'Csütörtök', value: 4 },
                    { label: 'Péntek', value: 5 },
                    { label: 'Szombat', value: 6 },
                    { label: 'Vasárnap', value: 7 },
                  ].map((day) => (
                    <label key={day.value} className="flex items-center space-x-2">
                      <input
                        type="checkbox"
                        className="rounded border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500"
                        checked={newProgram.weekdays.includes(day.value)}
                        onChange={(e) => {
                          const updatedWeekdays = e.target.checked
                            ? [...newProgram.weekdays, day.value]
                            : newProgram.weekdays.filter((weekday) => weekday !== day.value);
                          setNewProgram({ ...newProgram, weekdays: updatedWeekdays });
                        }}
                      />
                      <span>{day.label}</span>
                    </label>
                  ))}
                </div>
                <input
                  type="hidden"
                  placeholder="Comma-separated weekdays (e.g., 1,2,3)"
                  className="mt-1 block w-full rounded border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500"
                  value={newProgram.weekdays.join(',')}
                  onChange={(e) =>
                    setNewProgram({
                      ...newProgram,
                      weekdays: e.target.value.split(',').map(Number),
                    })
                  }
                />
              </label>
            </form>
            <div className="modal-actions mt-6 flex justify-end space-x-4">
              <button
                onClick={handleSaveProgram}
                className="btn btn-primary px-4 py-2 bg-indigo-600 text-white rounded hover:bg-indigo-700"
              >
                Mentés
              </button>
              <button
                onClick={handleModalClose}
                className="btn btn-secondary px-4 py-2 bg-gray-300 text-gray-700 rounded hover:bg-gray-400"
              >
                Mégsem
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
