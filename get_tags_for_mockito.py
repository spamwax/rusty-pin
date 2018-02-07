import json

f = open('tests/all_pins_mockito.json')
pins = json.load(f)

tags = {}

for pin in pins:
    if 'tags' in pin and pin['tags'].strip():
        t = pin['tags'].split()
        for tag in t:
            if tag not in tags:
                tags[tag] = 1
            else:
                tags[tag] = tags[tag] + 1

newtags = {}
for (k, v) in tags.items():
    newtags[k] = str(v)

print(json.dumps(newtags))
