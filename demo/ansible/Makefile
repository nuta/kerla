.PHONY: deploy
deploy:
	make -j8 -C ../kerla RELEASE=1
	cp ../kerla/kerla.x64.stripped.elf demo.elf
	ansible-playbook playbook.yml
