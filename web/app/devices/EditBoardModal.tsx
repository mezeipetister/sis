"use client";

import { useState } from "react";
import { updateBoard } from "../actions/board-actions";

type ZoneInfo = { id: string; name: string };

type Props = {
	device_id: string;
	initialName: string;
	initialZones: ZoneInfo[];
	onClose: () => void;
};

export default function EditBoardModal({ device_id, initialName, initialZones, onClose }: Props) {
	const [name, setName] = useState(initialName);
	const [zones, setZones] = useState(initialZones);

	const updateZoneName = (index: number, newName: string) => {
		setZones((prev) =>
			prev.map((z, i) => (i === index ? { ...z, name: newName } : z))
		);
	};

	async function handleSubmit(e: React.FormEvent) {
		e.preventDefault();
		await updateBoard(device_id, name, zones);
		onClose();
		location.reload(); // vagy router.refresh()
	}

	return (
		<div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
			<div className="bg-white rounded-lg p-6 max-w-md w-full shadow">
				<h2 className="text-xl font-bold mb-4">Edit board: {device_id}</h2>
				<form onSubmit={handleSubmit}>
					<div className="mb-4">
						<label className="block font-medium mb-1">Board name:</label>
						<input
							type="text"
							className="w-full border rounded px-3 py-2"
							value={name}
							onChange={(e) => setName(e.target.value)}
						/>
					</div>
					<div className="mb-4">
						<label className="block font-medium mb-1">Zones:</label>
						{zones.map((zone, index) => (
							<div key={zone.id} className="mb-2">
								<span className="text-sm text-gray-600">{zone.id}</span>
								<input
									type="text"
									className="w-full border rounded px-2 py-1 mt-1"
									value={zone.name}
									onChange={(e) => updateZoneName(index, e.target.value)}
								/>
							</div>
						))}
					</div>
					<div className="flex justify-end gap-2">
						<button type="button" onClick={onClose} className="px-4 py-2 bg-gray-300 rounded">Cancel</button>
						<button type="submit" className="px-4 py-2 bg-blue-600 text-white rounded">Save</button>
					</div>
				</form>
			</div>
		</div>
	);
}