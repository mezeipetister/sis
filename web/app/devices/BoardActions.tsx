"use client";

import { addBoard, removeBoard } from "../actions/board-actions";
import { useTransition } from "react";

type Props = {
	device_id: string;
	add: boolean;
};

export function BoardActions({ device_id, add }: Props) {
	const [isPending, startTransition] = useTransition();

	const handleClick = () => {
		startTransition(async () => {
			if (add) {
				await addBoard(device_id);
			} else {
				await removeBoard(device_id);
			}
			window.location.reload();
		});
	};

	return (
		<>
			<button
				onClick={() => {
					if (
						window.confirm(
							`Are you sure you want to ${add ? "add this device to" : "remove this device from"} boards?`
						)
					) {
						handleClick();
					}
				}}
				disabled={isPending}
				className={`px-4 py-2 text-sm font-medium text-white rounded ${!add ? "bg-red-600 hover:bg-red-700" : "bg-blue-600 hover:bg-blue-700"}`}
			>
				{!add ? "Remove from boards" : "Add to boards"}
			</button>
		</>
	);
}