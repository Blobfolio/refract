document.body.addEventListener('keypress', function(e) {
	if (e.keyCode == 32) {
		e.preventDefault();
		document.body.classList.toggle('ng');
	}
});

document.getElementById('image').addEventListener('click', function(e) {
	e.preventDefault();
	document.body.classList.toggle('ng');
});
