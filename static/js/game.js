'use strict';
import {Board} from './board.js';
import {Timer} from './timer.js';

//Web components
window.customElements.define('go-board', Board, {extends: 'canvas'});
window.customElements.define('go-timer', Timer, {extends: 'span'});

//Metadata
const url = new URL(document.URL);
const id = document.querySelector('meta[name="go:id"]').content;
const stoneName = document.querySelector('meta[name="go:stone"]').content;
const boardSize = parseInt(document.querySelector('meta[name="go:board-size"]').content, 10);
const handicap = parseInt(document.querySelector('meta[name="go:handicap"]').content, 10);
let stone;
switch (stoneName) {
	case 'black':
		stone = 1;
		break;
	case 'white':
		stone = 2;
		break;
}

//UI elements
const statusText = document.getElementById('status');
//Board
const board = document.getElementById('board');
board.board_size = boardSize;
board.cursor.stone = stone;
board.draw();
//Buttons
const handicapButtons = document.getElementById('handicap-buttons');
const playButtons = document.getElementById('play-buttons');
//Timers
const blackTimer = document.getElementById('black-timer');
const whiteTimer = document.getElementById('white-timer');
//Scoring
const score = document.getElementById('score');
const blackScore = document.getElementById('black-score');
const whiteScore = document.getElementById('white-score');
const scoreStatement = document.getElementById('score-statement');
score.style.display = 'none';

//Data
const socket = new WebSocket(`wss://${url.host}/ws/${id}/${stoneName}`);
let handicaps = [];

function suspend() {
	board.enabled = false;
	board.clickListeners.clear();
	for (const button of handicapButtons.children)
		button.setAttribute('disabled', '');
	for (const button of playButtons.children)
		button.setAttribute('disabled', '');
	blackTimer.pause();
	whiteTimer.pause();
}

function handicapPlacement(x, y) {
	const index = board.size * y + x;
	if (board.moves[index] && handicaps.length < handicap) {
		handicaps.push(index);
		board.stones[index] = stone;
		//Button state
		handicapButtons.children[1].removeAttribute('disabled');
		if (handicaps.length == handicap)
			handicapButtons.children[0].removeAttribute('disabled');
	}
}

function playPlacement(x, y) {
	const index = board.size * y + x;
	if (board.moves[index]) {
		//Play stone
		board[index] = stone;
		socket.send(JSON.stringify({
			action: 'play',
			position: index
		}));
		board.draw();
		suspend();
	}
}

//Handicap play button
handicapButtons.children[0].addEventListener('click', event => {
	socket.send(JSON.stringify({
		action: 'handicap',
		positions: handicaps
	}));
	suspend();
});

//Handicap reset button
handicapButtons.children[1].addEventListener('click', event => {
	handicapButtons.children[0].setAttribute('disabled', '');
	handicapButtons.children[1].setAttribute('disabled', '');
	for (const x of handicaps)
		board.stones[x] = 0;
	handicaps = [];
	board.draw();
});

//Pass button
playButtons.children[0].addEventListener('click', event => {
	socket.send(JSON.stringify({action: 'pass'}));
	suspend();
});

//Resign button
playButtons.children[1].addEventListener('click', event => {
	socket.send(JSON.stringify({action: 'resign'}));
	suspend();
});

//WebSocket events
socket.addEventListener('message', event => {
	suspend();
	const frame = JSON.parse(event.data);
	console.log(frame);
	board.update(frame);
	blackTimer.update(frame.black_time);
	whiteTimer.update(frame.white_time);
	switch (frame.turn) {
		case 'wait':
			statusText.innerText = 'Waiting for players';
			blackTimer.resume();
			whiteTimer.resume();
			break;
		case 'handicap':
			statusText.innerText = 'Black to play handicap'
			blackTimer.resume();
			break;
		case 'black':
			statusText.innerText = 'Black to play'
			blackTimer.resume();
			break;
		case 'white':
			statusText.innerText = 'White to play'
			whiteTimer.resume();
			break;
		case 'end':
			statusText.innerText = 'Game over'
			score.style.display = 'block';
			blackScore.innerText = frame.black_score;
			whiteScore.innerText = frame.white_score;
			if (frame.black_score > frame.white_score)
				scoreStatement.innerText = `Black wins by +${frame.black_score - frame.white_score}`;
			else if (frame.white_score > frame.black_score)
				scoreStatement.innerText = `White wins by +${frame.white_score - frame.black_score}`;
			else scoreStatement.innerText = 'Draw';
			break;
	}
	//Interactions
	if (stone === 1 && frame.turn === 'handicap') {
		//Handicap
		board.enabled = true;
		board.clickListeners.add(handicapPlacement);
	} else if (stoneName === frame.turn) {
		//Play
		board.enabled = true;
		board.clickListeners.add(playPlacement);
		for (const button of playButtons.children)
			button.removeAttribute('disabled');
	}
	board.draw();
});
socket.addEventListener('error', event => {
	statusText.textContent = 'Connection lost';
});
