"use client";

import { useState } from "react";
import { setProgram } from "@/app/actions/schedule-actions";
import { ZoneInfo } from "../actions/board-actions";

type Props = {
	program: any;
	zones: ZoneInfo[];
	onClose: () => void;
};

export default function EditProgramModal({ program, zones, onClose }: Props) {
	const [name, setName] = useState(program.name);
	const [startTime, setStartTime] = useState(program.start_time);
	const [weekdays, setWeekdays] = useState<number[]>(program.weekdays);

	const initialSelectedZones = zones
		.filter((z) =>
			program.zones.some((pz: any) => pz.zone_ids.includes(z.id))
		)
		.map((z) => ({ zone_ids: [z.id], duration_seconds: program.zones.find((pz: any) => pz.zone_ids.includes(z.id))?.duration_seconds || 300 }));

	const [selectedZones, setSelectedZones] = useState(initialSelectedZones);

	const toggleWeekday = (day: number) => {
		setWeekdays((prev) =>
			prev.includes(day) ? prev.filter((d) => d !== day) : [...prev, day]
		);
	};

	const handleZoneToggle = (zoneId: string) => {
		setSelectedZones((prev) => {
			const exists = prev.find((z) => z.zone_ids.includes(zoneId));
			if (exists) {
				return prev.filter((z) => !z.zone_ids.includes(zoneId));
			} else {
				return [...prev, { zone_ids: [zoneId], duration_seconds: 300 }];
			}
		});
	};

	const handleSubmit = async () => {
		await setProgram({
			id: program.id,
			name,
			start_time: startTime,
			weekdays,
			active: program.active,
			zones: selectedZones,
		});
		onClose();
		location.reload();
	};

	return (
		<div className="fixed inset-0 bg-black bg-opacity-40 flex items-center justify-center z-50">
			<div className="bg-white p-6 rounded shadow max-w-md w-full">
				<h2 className="text-lg font-bold mb-4">Program szerkesztése</h2>
				<input
					placeholder="Név"
					className="border px-3 py-2 mb-2 w-full"
					value={name}
					onChange={(e) => setName(e.target.value)}
				/>
				<input
					type="time"
					className="border px-3 py-2 mb-2 w-full"
					value={startTime}
					onChange={(e) => setStartTime(e.target.value)}
				/>
				<div className="mb-4">
					<label className="block font-medium mb-1">Napok:</label>
					{[0, 1, 2, 3, 4, 5, 6].map((d) => (
						<button
							key={d}
							onClick={() => toggleWeekday(d)}
							className={`px-2 py-1 border rounded mr-1 mb-1 ${weekdays.includes(d) ? "bg-blue-500 text-white" : "bg-white"}`}
						>
							{"MTWTFSS"[d]}
						</button>
					))}
				</div>
				<div className="mb-4">
					<label className="block font-medium mb-1">Zónák:</label>
					<div className="flex flex-col gap-1">
						{zones.map((zone) => (
							<label key={zone.id} className="flex items-center gap-2">
								<input
									type="checkbox"
									checked={selectedZones.some((z) => z.zone_ids.includes(zone.id))}
									onChange={() => handleZoneToggle(zone.id)}
								/>
								{zone.name || zone.id}
							</label>
						))}
					</div>
				</div>
				<button
					onClick={handleSubmit}
					className="bg-green-600 text-white px-4 py-2 rounded"
				>
					Mentés
				</button>
				<button onClick={onClose} className="ml-2 text-gray-600">
					Mégse
				</button>
			</div>
		</div>
	);
}