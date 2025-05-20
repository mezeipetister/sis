"use client";

import { useState } from "react";
import { setProgram } from "@/app/actions/schedule-actions";

export default function NewProgramModal() {
	const [open, setOpen] = useState(false);
	const [name, setName] = useState("");
	const [startTime, setStartTime] = useState("06:00");
	const [weekdays, setWeekdays] = useState<number[]>([]);
	const [zones, setZones] = useState([]);

	const toggleWeekday = (day: number) => {
		setWeekdays((prev) =>
			prev.includes(day) ? prev.filter((d) => d !== day) : [...prev, day]
		);
	};

	const handleSubmit = async () => {
		await setProgram({
			id: name.toLowerCase().replace(/\s+/g, "_"),
			name,
			start_time: startTime,
			weekdays,
			active: true,
			zones,
		});
		setOpen(false);
		location.reload();
	};

	return (
		<>
			<button
				className="px-4 py-2 bg-blue-600 text-white rounded"
				onClick={() => setOpen(true)}
			>
				Új program
			</button>

			{open && (
				<div className="fixed inset-0 bg-black bg-opacity-40 flex items-center justify-center z-50">
					<div className="bg-white p-6 rounded shadow max-w-md w-full">
						<h2 className="text-lg font-bold mb-4">Új program hozzáadása</h2>
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
									{"VHSCPPS"[d]}
								</button>
							))}
						</div>
						<button
							onClick={handleSubmit}
							className="bg-green-600 text-white px-4 py-2 rounded"
						>
							Mentés
						</button>
						<button onClick={() => setOpen(false)} className="ml-2 text-gray-600">
							Mégse
						</button>
					</div>
				</div>
			)}
		</>
	);
}