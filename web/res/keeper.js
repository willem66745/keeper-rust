// Show loader and disable the rest of the user interface
function show_loader() {
    $("#foo").addClass('ui-disabled');
    $.mobile.loading("show", {
        text: "loading",
        textVisible: true
    });
}

// Hide loader and activate the rest of the user interface
function hide_loader() {
    $.mobile.loading("hide");
    $("#foo").removeClass('ui-disabled');
}

// Retrieve all switch configurations and update web user interface
//
// See: http://demos.jquerymobile.com/1.4.5/listview/
function load_switches() {
    show_loader();

    $.post("/api/switches", function(data) {
        var items = data.length;

        $("#content").empty();
        $("#content").hide();

        $.each(data, function(index, object) {
            var li = $("#content").append('<li>' + object + '</li>');

            li = li.find("li").last();

            $.ajax({
                url: "/api/get/" + object,
                type: "POST",
                async: false,
                timeout: 2000
            }).done(function(data) {
                //console.debug(object);
                //console.debug(data);
                if (data.switch == true) {
                    // apply light theme when switch is enabled
                    li.attr("data-theme", "a");
                }
                else {
                    // apply dark theme when switch is enabled
                    li.attr("data-theme", "b");
                }
            }).fail(function() {
            }).always(function() {
                items--;
                if (items == 0) {
                    hide_loader();
                }
            });
        });

        $("#content").listview("refresh");
        $("#content").show();
    });
}

// Load web page and set event handlers
$(function() {
    load_switches();

    $("#refresh").click(function(event) {
        event.preventDefault();
        load_switches();
    });
});
