CREATE TABLE `share` (
	`id` text PRIMARY KEY NOT NULL,
	`created_at` integer NOT NULL,
	`width` integer NOT NULL,
	`height` integer NOT NULL,
	`skyline` text NOT NULL,
	`box` text,
	`result` text NOT NULL
);
