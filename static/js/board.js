'use strict';

class Cursor {
	constructor(stone) {
		this.stone = stone;
		this.enabled = false;
		this.position = [0, 0];
	}
}

export class Board {
	constructor(canvas, tileset, stone) {
		//HTML elements
		this.canvas = canvas;
		this.ctx = canvas.getContext('2d');
		this.tileset = tileset;
		//Board
		this.size = 0;
		this.stones = new Uint8Array(this.size * this.size);
		this.moves = new Uint8Array(this.size * this.size);
		//Cursor
		this.cursor = new Cursor(stone);
	}
	resize(size) {
		this.size = size;
		this.stones = new Uint8Array(this.size * this.size);
		this.moves = new Uint8Array(this.size * this.size);
		//Canvas
		this.canvas.width = 16 * size;
		this.canvas.height = 16 * size;
		this.ctx = this.canvas.getContext('2d');
		this.ctx.imageSmoothingEnabled = false;
	}
	drawBoard() {
		//Draw board
		this.ctx.translate(0.5, 0.5);
		//Gray lines
		this.ctx.strokeStyle = '#202020';
		for (let i = 0; i < this.size; ++i) {
			//Vertical
			this.ctx.beginPath();
			this.ctx.moveTo(16 * i + 7, 7);
			this.ctx.lineTo(16 * i + 7, 16 * this.size - 8);
			this.ctx.stroke();
			//Horizontal
			this.ctx.beginPath();
			this.ctx.moveTo(7, 16 * i + 7);
			this.ctx.lineTo(16 * this.size - 8, 16 * i + 7);
			this.ctx.stroke();
		}
		//Black lines
		this.ctx.strokeStyle = '#000000';
		for (let i = 0; i < this.size; ++i) {
			//Vertical
			this.ctx.beginPath();
			this.ctx.moveTo(16 * i + 8, 8);
			this.ctx.lineTo(16 * i + 8, 16 * this.size - 8);
			this.ctx.stroke();
			//Horizontal
			this.ctx.beginPath();
			this.ctx.moveTo(8, 16 * i + 8);
			this.ctx.lineTo(16 * this.size - 8, 16 * i + 8);
			this.ctx.stroke();
		}
		this.ctx.resetTransform();
	}
	drawStones() {
		for (let y = 0; y < this.size; ++y) {
			for (let x = 0; x < this.size; ++x) {
				const stone = this.stones[this.size * y + x];
				switch (stone) {
					case 1:
						this.ctx.drawImage(this.tileset, 0, 0, 8, 8, 16 * x, 16 * y, 16, 16);
						break;
					case 2:
						this.ctx.drawImage(this.tileset, 8, 0, 8, 8, 16 * x, 16 * y, 16, 16);
						break;
				}
			}
		}
	}
	drawCursor() {
		if (this.cursor.enabled) {
			const x = this.cursor.position[0];
			const y = this.cursor.position[1];
			const index = this.size * y + x;
			const stone = this.stones[index];
			const legal = this.moves[index];
			if (stone === 0 && legal) {
				this.ctx.globalAlpha = 0.5;
				const tileOffset = this.cursor.stone === 'black' ? 0 : 8;
				this.ctx.drawImage(
					this.tileset,
					tileOffset, 0,
					8, 8,
					16 * x, 16 * y,
					16, 16
				);
				this.ctx.globalAlpha = 1;
			}
		}
	}
	draw() {
		this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
		this.drawBoard();
		this.drawStones();
		this.drawCursor();
	}
	place(x, y, stone) {
		switch (stone) {
			case 'black':
				stone = 1;
				break;
			case 'white':
				stone = 2;
				break;
		}
		this.stones[this.size * y + x] = stone;
	}
	update(frame) {
		this.resize(frame.board_size);
		this.stones = new Uint8Array(frame.board);
		this.moves = new Uint8Array(frame.moves);
	}
}
